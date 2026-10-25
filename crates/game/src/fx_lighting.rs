//! Light on lit FXR particles (#324), after the game's effect shaders.
//!
//! The game lights a lit billboard per vertex in its geometry shader
//! (`GXFfxtessellateSoftTexture.gpo`, the same body in the `.dpo` domain shaders) and finishes in
//! the pixel shader (`GXFfxtessellateBlendSoftTexture.ppo`), both disassembled with
//! `d3dcompiler_47`:
//!
//! - Vertex (`TEXCOORD6`): the irradiance volume's constant band (`g_IrradianceVolumeW`, decoded
//!   `2 (rgb - 0.5) 2^(20 (a - 0.5))`, clamped at 0, times `g_ds_irradianceVolumeDiffuseColorScale`,
//!   volumes A and B blended by `g_ds_irradianceVolumeInterpRatio`) plus every tile point and spot
//!   light at the vertex: `Diff.rgb * saturate(1 - d^2 / R^2) / (1 + 0.7 (d b)^2) *
//!   (0.75 t^2 + 0.25)` with `t = 1.4 / (1.4 + d b)`. That is the deferred point light's wrap
//!   term with no normal (N.L averaged to 1/4), see docs/MAP-FX.md.
//! - Pixel, when `g_ps_bEnableLighting`: `saturate(texture x colour) x (sun x shadow + vertex)`,
//!   the sun being the light set's directional colour 0 (`FC_DirLight[envCbIndex].Col_0`) times
//!   `g_ps_envCbColScale` and a light-mask factor. There is no N.L: a billboard takes the sun
//!   whole.
//!
//! Here it is evaluated on the CPU once per particle (the game does it per vertex, which is the
//! same for a sprite small against the light's range). Inferred or open: the light mask,
//! `g_ps_envCbColScale` and the volume colour scale are taken as 1 (their CPU uploads are not
//! traced); overlapping volumes blend by their edge fade as
//! the map shader's `ambient_light` does, not by the game's A/B ratio; the GS's upper-hemisphere
//! branch (`g_ds_isIVUpper`, which adds the Y band) is not taken; spot lights are left out.

use std::sync::Arc;

use bevy::prelude::*;

/// The native irradiance volumes' constant (W) band, kept on the CPU for particle lighting.
/// Shared, so the particle renderer holds it across frames without copying the voxels.
#[derive(Resource, Default, Clone)]
pub struct IrradianceCpu {
    pub volumes: Arc<[GiVolume]>,
}

/// One volume: world to native UV box space, its world bounds, the record's full voxel size
/// (for the edge fade) and the W plane's size and RGBA8 voxels (x fastest, then y, then z).
pub struct GiVolume {
    pub uv: Mat4,
    pub min: Vec3,
    pub max: Vec3,
    pub dims: UVec3,
    pub w_dims: UVec3,
    pub w: Vec<u32>,
}

impl GiVolume {
    fn texel(&self, x: u32, y: u32, z: u32) -> Vec4 {
        let d = self.w_dims;
        debug_assert_eq!(self.w.len(), d.element_product() as usize);
        let b = self.w[((z * d.y + y) * d.x + x) as usize].to_le_bytes();
        Vec4::new(b[0] as f32, b[1] as f32, b[2] as f32, b[3] as f32) / 255.
    }

    /// The W band at box-space `uv` (0 to 1 across the volume): trilinear filtered RGBA8 with
    /// texel centres at i + 0.5 and clamped to the edge centres, decoded after filtering as the
    /// GPU does. The CPU twin of `gi_sample` (axis 3) in `map_fog_surface.wesl`.
    fn sample(&self, uv: Vec3) -> Vec3 {
        let d = self.w_dims.as_vec3();
        let p = (uv * d).clamp(Vec3::splat(0.5), d - 0.5) - 0.5;
        let i = p.floor();
        let f = p - i;
        let i = i.as_uvec3();
        let hi = (i + 1).min(self.w_dims - 1);
        let mut acc = Vec4::ZERO;
        for (dz, wz) in [(i.z, 1. - f.z), (hi.z, f.z)] {
            for (dy, wy) in [(i.y, 1. - f.y), (hi.y, f.y)] {
                for (dx, wx) in [(i.x, 1. - f.x), (hi.x, f.x)] {
                    acc += self.texel(dx, dy, dz) * (wx * wy * wz);
                }
            }
        }
        (2. * (acc.xyz() - 0.5) * (20. * (acc.w - 0.5)).exp2()).max(Vec3::ZERO)
    }

    fn overlaps(&self, min: Vec3, max: Vec3) -> bool {
        self.min.cmple(max).all() && self.max.cmpge(min).all()
    }
}

/// Native irradiance at `pos` (the constant band) from `volumes` in order, 0 outside every one.
/// The CPU twin of `ambient_light` in `map_fog_surface.wesl`, W band only.
fn irradiance<'a>(volumes: impl Iterator<Item = &'a GiVolume>, pos: Vec3) -> Vec3 {
    let mut result = Vec3::ZERO;
    for v in volumes {
        if pos.cmplt(v.min).any() || pos.cmpgt(v.max).any() {
            continue;
        }
        let uv = v.uv.transform_point3(pos);
        let edge = uv.min(1. - uv) * v.dims.as_vec3();
        let x = edge.min_element().clamp(0., 1.);
        let fade = x * x * (3. - 2. * x);
        if fade <= 0. {
            continue;
        }
        result = result.lerp(v.sample(uv), fade);
    }
    result
}

/// One local light as the particle shader reads it, in native units.
#[derive(Clone, Copy, Debug)]
pub struct NativeLight {
    pub position: Vec3,
    pub color: Vec3,
    pub range: f32,
    /// `b`: 1 / the falloff distance.
    pub b: f32,
}

impl NativeLight {
    /// The native point light's distance curve at `d`, `saturate(1 - d^2 / R^2) /
    /// (1 + 0.7 (d b)^2)`, shared by the map's deferred light (`native_clustered_light` in
    /// `map_material.wesl`) and the effect geometry shader.
    pub fn falloff(&self, d: f32) -> f32 {
        let fall = (1. - (d / self.range).powi(2)).clamp(0., 1.);
        let db = d * self.b;
        fall / (1. + 0.7 * db * db)
    }

    /// The geometry shader's point light at distance `d`, with no normal: the distance curve
    /// times the wrap term `0.75 t^2 + 0.25`, `t = 1.4 / (1.4 + d b)`.
    pub fn at(&self, d: f32) -> Vec3 {
        let fall = self.falloff(d);
        if fall <= 0. {
            return Vec3::ZERO;
        }
        let t = 1.4 / (1.4 + d * self.b);
        self.color * (fall * (0.75 * t * t + 0.25))
    }

    /// Whether the light reaches any point of the box `min..max`.
    fn reaches(&self, min: Vec3, max: Vec3) -> bool {
        self.position
            .clamp(min, max)
            .distance_squared(self.position)
            < self.range * self.range
    }
}

/// Whether lit particles take the sun shadowed (#366); `--ab particle-shadow=on,off` flips it.
#[derive(Resource, Clone, Copy, PartialEq, Eq, Debug)]
pub struct ParticleShadows(pub bool);

impl Default for ParticleShadows {
    fn default() -> Self {
        Self(true)
    }
}

/// The shadowed sun (#366). The game's lit effect pixel shaders
/// (`GXFfxtessellateBlendSoftMultiTexture.ppo` and the other lit `.ppo`, d3dcompiler_47) scale the
/// sun term by `1 + g_ps_BillboardShadowIntensity (shadow - 1)`, `shadow` read from
/// `g_shadowVolumeTexture`, the volumetric fog's sun-shadow volume. That `cb0[35].x` is
/// `g_ps_BillboardShadowIntensity` is in the shader's reflection; that the CPU fills it from the
/// appearance's `shadowDarkness` is inferred (same meaning and range, not traced in the exe).
/// Here the light reaches the shader already summed per particle, so each vertex carries, per
/// channel, `shadowDarkness x sun / light`: the share of its colour the shader shadows
/// (the tests check the product on the CPU). The shader takes the shadow from Bevy's sun
/// cascades (`fxr.wesl` `sun_shadow_factor`), as the effect pass has no fog volume bound.
pub fn sun_shade(sun: Vec3, light: Vec3, darkness: f32) -> Vec3 {
    let d = darkness.clamp(0., 1.);
    Vec3::from_array(std::array::from_fn(|i| {
        if light[i] > 1e-6 {
            d * (sun[i] / light[i]).clamp(0., 1.)
        } else {
            0.
        }
    }))
}

/// The sun's inputs from [`crate::lighting::MapLighting`]: light set, hour, gparam source and
/// lighting revision, as `update_hemisphere` keys its upload.
type SunKey = (i32, f32, String, u32);

/// What lights a lit particle, in native units (multiply by [`crate::lighting::ENV_PER_UNIT`]
/// for Bevy). Kept across frames: [`Self::gather`] refreshes it, [`Self::cull`] picks what
/// reaches one node and [`Self::at`] evaluates a particle of that node.
#[derive(Default)]
pub struct ParticleLighting {
    /// The light set's directional light 0 diffuse colour times its W.
    pub sun: Vec3,
    sun_key: Option<SunKey>,
    /// Every point light this frame.
    pub lights: Vec<NativeLight>,
    pub gi: IrradianceCpu,
    /// The lights and volumes (indices into `gi`) reaching the node culled last.
    near: Vec<NativeLight>,
    near_gi: Vec<usize>,
    /// [`ParticleShadows`] this frame.
    pub shadowed: bool,
}

impl ParticleLighting {
    /// Refreshes the sun (only when the light set in force changes), the irradiance volumes and
    /// the point lights.
    pub fn gather(
        &mut self,
        lighting: Option<&crate::lighting::MapLighting>,
        gi: Option<&IrradianceCpu>,
        points: impl Iterator<Item = NativeLight>,
        shadowed: bool,
    ) {
        self.shadowed = shadowed;
        let key = lighting.map(|l| (l.light_set, l.hour, l.source.clone(), l.revision));
        if key != self.sun_key {
            self.sun = lighting.map_or(Vec3::ZERO, |l| {
                l.surface_light(None).direct[1].xyz() / crate::lighting::SCENE_TO_BEVY
            });
            self.sun_key = key;
        }
        match gi {
            Some(gi) if !Arc::ptr_eq(&gi.volumes, &self.gi.volumes) => self.gi = gi.clone(),
            None if !self.gi.volumes.is_empty() => self.gi = IrradianceCpu::default(),
            _ => {}
        }
        self.lights.clear();
        self.lights.extend(points);
    }

    /// Picks the lights and volumes reaching the box `min..max`, the bounds of a lit node's
    /// particle centres (where [`Self::at`] evaluates, so no margin is needed).
    pub fn cull(&mut self, min: Vec3, max: Vec3) {
        self.near.clear();
        self.near
            .extend(self.lights.iter().filter(|l| l.reaches(min, max)));
        self.near_gi.clear();
        self.near_gi.extend(
            self.gi
                .volumes
                .iter()
                .enumerate()
                .filter(|(_, v)| v.overlaps(min, max))
                .map(|(i, _)| i),
        );
    }

    /// Per channel, the share of `light` (from [`Self::at`]) the shader shadows, see [`sun_shade`];
    /// zero when [`ParticleShadows`] is off.
    pub fn shade(&self, light: Vec3, darkness: f32) -> Vec3 {
        if self.shadowed {
            sun_shade(self.sun, light, darkness)
        } else {
            Vec3::ZERO
        }
    }

    /// The native light on a lit particle at `pos`, from what [`Self::cull`] picked.
    pub fn at(&self, pos: Vec3) -> Vec3 {
        let gi = irradiance(self.near_gi.iter().map(|&i| &self.gi.volumes[i]), pos);
        let mut l = self.sun + gi;
        for light in &self.near {
            l += light.at(light.position.distance(pos));
        }
        l
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn point_light_wrap_without_normal() {
        let l = NativeLight {
            position: Vec3::ZERO,
            color: Vec3::ONE,
            range: 5.,
            b: 1.,
        };
        // At the centre: fall 1, t 1, so the full colour.
        assert!((l.at(0.).x - 1.).abs() < 1e-6);
        // Past the range: nothing.
        assert_eq!(l.at(5.), Vec3::ZERO);
        // At 1 m: (1 - 1/25) / 1.7 x (0.75 (1.4/2.4)^2 + 0.25).
        let t: f32 = 1.4 / 2.4;
        let want = 0.96 / 1.7 * (0.75 * t * t + 0.25);
        assert!((l.at(1.).x - want).abs() < 1e-6);
    }

    fn flat_volume() -> IrradianceCpu {
        // rgb 0.75, a 0.5 + 1/20 (so 2^1): 2 x 0.25 x 2 = 1.
        let a = ((0.5 + 0.05) * 255f32).round() as u32;
        let rgb = (0.75 * 255f32).round() as u32;
        let texel = rgb | rgb << 8 | rgb << 16 | a << 24;
        IrradianceCpu {
            volumes: vec![GiVolume {
                uv: Mat4::from_scale(Vec3::splat(0.1)),
                min: Vec3::ZERO,
                max: Vec3::splat(10.),
                dims: UVec3::splat(4),
                w_dims: UVec3::splat(4),
                w: vec![texel; 64],
            }]
            .into(),
        }
    }

    #[test]
    fn irradiance_decodes_a_flat_volume() {
        let gi = flat_volume();
        let v = irradiance(gi.volumes.iter(), Vec3::splat(5.));
        assert!((v.x - 1.).abs() < 0.03, "{v}");
        assert_eq!(irradiance(gi.volumes.iter(), Vec3::splat(11.)), Vec3::ZERO);
    }

    /// Culling to a node's bounds keeps exactly the lights and volumes that can reach it, so a
    /// particle inside lights the same as with nothing culled.
    #[test]
    fn culling_keeps_what_reaches_the_node() {
        let light = |x: f32| NativeLight {
            position: Vec3::new(x, 5., 5.),
            color: Vec3::ONE,
            range: 3.,
            b: 1.,
        };
        let mut l = ParticleLighting::default();
        l.gather(
            None,
            Some(&flat_volume()),
            [light(4.), light(20.)].into_iter(),
            true,
        );
        let pos = Vec3::splat(5.);
        l.cull(pos, pos);
        assert_eq!(l.near.len(), 1);
        assert_eq!(l.near_gi, [0]);
        let want = irradiance(l.gi.volumes.iter(), pos) + light(4.).at(1.);
        assert!((l.at(pos) - want).length() < 1e-6);
        let far = Vec3::splat(30.);
        l.cull(far, far);
        assert!(l.near.is_empty() && l.near_gi.is_empty());
        assert_eq!(l.at(far), Vec3::ZERO);
    }

    /// The light after the shader's shadowing (`fx_shadow.wesl` `sun_shadow_factor`),
    /// `light (1 + shade (shadow - 1))`.
    fn shaded_light(sun: Vec3, light: Vec3, darkness: f32, shadow: f32) -> Vec3 {
        light * (Vec3::ONE + sun_shade(sun, light, darkness) * (shadow - 1.))
    }

    /// #366: the game's `sun (1 + shadowDarkness (shadow - 1))`. Full shadow with darkness 0.9
    /// leaves 10% of the sun, darkness 0 leaves it whole, and the rest of the light is untouched.
    #[test]
    fn shadowed_sun_keeps_one_minus_darkness_of_the_sun() {
        let sun = Vec3::new(2., 1., 0.5);
        let rest = Vec3::new(0.25, 0.5, 1.);
        let light = sun + rest;
        let near = |a: Vec3, b: Vec3| (a - b).length() < 1e-6;
        assert!(near(shaded_light(sun, light, 0.9, 0.), sun * 0.1 + rest));
        assert!(near(shaded_light(sun, light, 0., 0.), light));
        assert!(near(shaded_light(sun, light, 0.9, 1.), light));
        assert!(near(shaded_light(sun, light, 1., 0.5), sun * 0.5 + rest));
        // No sun, or no light at all: nothing to shadow.
        assert_eq!(sun_shade(Vec3::ZERO, rest, 0.9), Vec3::ZERO);
        assert_eq!(sun_shade(sun, Vec3::ZERO, 0.9), Vec3::ZERO);
        // The `--ab particle-shadow=off` switch leaves every channel unshaded.
        let mut l = ParticleLighting {
            sun,
            shadowed: false,
            ..Default::default()
        };
        assert_eq!(l.shade(light, 0.9), Vec3::ZERO);
        l.shadowed = true;
        assert!(near(l.shade(light, 0.9), sun_shade(sun, light, 0.9)));
    }
}
