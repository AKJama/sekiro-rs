"""Spatial enrichment and reference-only review artifacts."""
import json
from pathlib import Path
import numpy as np
from PIL import Image, ImageDraw


def collision(path):
    data = path.read_bytes(); offset = 0
    def varint():
        nonlocal offset
        value = shift = 0
        while True:
            b = data[offset]; offset += 1
            value |= (b & 127) << shift
            if b < 128:
                return value
            shift += 7
            if shift > 35:
                raise ValueError('Invalid postcard varint')
    n = varint()
    vertices = np.frombuffer(data, dtype='<f4', count=n*3, offset=offset).reshape(-1,3)
    offset += n*12
    n = varint()
    indices = np.fromiter((varint() for _ in range(n)), dtype=np.uint32, count=n).reshape(-1,3)
    count = varint()
    for _ in range(count):
        varint()
    if offset != len(data) or count != n//3:
        raise ValueError('Unexpected CollisionMesh serialization')
    return vertices[indices]


def floor_at(tri, lo, hi, position, tolerance=0.0):
    x,y,z = position
    ids=np.where((lo[:,0]<=x)&(hi[:,0]>=x)&(lo[:,2]<=z)&(hi[:,2]>=z)&(lo[:,1]<=y+tolerance))[0]
    t=tri[ids].astype(float)
    if not len(t):
        return None
    a=t[:,1]-t[:,0]; b=t[:,2]-t[:,0]
    det=a[:,0]*b[:,2]-a[:,2]*b[:,0]
    safe=np.abs(det)>1e-8
    det=np.where(safe,det,1)
    dx=x-t[:,0,0]; dz=z-t[:,0,2]
    u=(dx*b[:,2]-dz*b[:,0])/det;v=(a[:,0]*dz-a[:,2]*dx)/det
    h=t[:,0,1]+u*a[:,1]+v*b[:,1]
    good=safe&(u>=-1e-5)&(v>=-1e-5)&(u+v<=1.00001)&(h<=y+tolerance)
    return float(h[good].max()) if good.any() else None


def enrich(entries, maps, out, folder):
    idols=[]; geometry={}
    for block in ['m11_00','m11_01']:
        root=maps/(block+'_00_00');layout=json.loads((root/'layout.json').read_text())
        for p in layout['parts']:
            if p['kind']=='Object' and (p.get('model') in ('o005000', 'o005500') or p.get('entity_id')==1101950):
                idols.append(dict(name='Dilapidated Temple' if p['entity_id']==1101950 else f"Sculptor's Idol {p['entity_id']}",entity_id=p['entity_id'],part=p['name'],map_block=block,position=p['translation']))
        tri=collision(root/'collision.bin');geometry[block]=(tri,tri.min(1),tri.max(1))
        print(f'{block}: {len(tri)} collision triangles',flush=True)
    for e in entries:
        if not e['position']:
            continue
        pos=np.array(e['position'])
        near=min(idols,key=lambda a:np.linalg.norm(pos-a['position']))
        e['nearest_landmark']={**near,'distance':float(np.linalg.norm(pos-near['position']))}
        floors={b:floor_at(*g,pos,tolerance=.15) for b,g in geometry.items()}
        supported={b:h for b,h in floors.items() if h is not None}
        if supported:
            block=max(supported,key=supported.get)
            # Identical connected geometry cannot establish the owning map.
            tied=[b for b,h in supported.items() if abs(h-supported[block])<.05]
            if near['map_block'] in tied:
                block=near['map_block']
            e['map_block']=block
            e['support_height_difference']=float(pos[1]-supported[block])
            below=floor_at(*geometry[block],pos)
            e['floor_y']=below
            e['height_above_floor']=float(pos[1]-below) if below is not None else None
            e['map_assignment']={'method':'highest_collision_below; nearest_idol_breaks_shared_floor_ties','candidate_floor_y':floors,'shared_floor_blocks':tied}
        else:
            e['map_block']='other';e['map_assignment']={'method':'no_collision_below','candidate_floor_y':floors}
    (out/'landmarks.json').write_text(json.dumps(idols,indent=2)+'\n')
    (out/'index.json').write_text(json.dumps(entries,indent=2)+'\n')
    for block in ['m11_00','m11_01','other','unpositioned']:
        group=[e for e in entries if (e['map_block'] or 'unpositioned')==block]
        for page,start in enumerate(range(0,len(group),72),1):
            subset=group[start:start+72];sheet=Image.new('RGB',(1600,((len(subset)+7)//8)*132),'#111820');d=ImageDraw.Draw(sheet)
            for j,e in enumerate(subset):
                with Image.open(folder/e['file']) as im:
                    im.thumbnail((200,112));x=j%8*200;y=j//8*132;sheet.paste(im,(x,y))
                d.rectangle((x,y,x+42,y+16),fill='black');d.text((x+2,y+2),str(e['index']),fill='white')
                d.text((x+2,y+114),e['capture_time'][11:],fill='white')
            sheet.save(out/f'contact-{block}-{page:02}.jpg',quality=92)
    valid=[e for e in entries if e['position']]
    points=np.array([e['position'] for e in valid]);low=points[:,[0,2]].min(0)-50;high=points[:,[0,2]].max(0)+50
    scale=min(1900/(high-low));size=np.ceil((high-low)*scale).astype(int)+100
    canvas=Image.new('RGB',tuple(size),'#111820');d=ImageDraw.Draw(canvas)
    def px(v):
        return tuple(((v-low)*scale+50).astype(int))
    for block,g in geometry.items():
        tri=g[0];center=tri.mean(1)[:,[0,2]]
        ids=np.where(((center>=low)&(center<=high)).all(1))[0]
        color='#274454' if block=='m11_00' else '#495238'
        projected=((tri[ids][:,:,[0,2]]-low)*scale+50).astype(np.int32)
        for t in projected.tolist():
            d.line([tuple(v) for v in t]+[tuple(t[0])],fill=color,width=1)
        print(f'Drew {block} collision outline',flush=True)
    for e in valid:
        x,y=px(np.array(e['position'])[[0,2]]);color='#ff7e55' if e['map_block']=='m11_00' else '#72caff'
        d.ellipse((x-3,y-3,x+3,y+3),fill=color)
        d.text((x+4,y-7),str(e['index']),fill=color)
    for a in idols:
        x,y=px(np.array(a['position'])[[0,2]]);d.rectangle((x-4,y-4,x+4,y+4),outline='white',width=2)
    d.text((12,12),'Bevy X right, Z down | orange Outskirts, blue Castle | white squares: authored idols',fill='white')
    missing=', '.join(str(e['index']) for e in entries if not e['position'])
    d.text((12,30),f'Unpositioned source images: {missing or "none"}; coincident captures overlap',fill='white')
    canvas.save(out/'top-down.png')
    (out/'top-down-transform.json').write_text(json.dumps(dict(horizontal_axes=['bevy_x','bevy_z'],minimum=low.tolist(),pixels_per_metre=float(scale),pixel_margin=50),indent=2)+'\n')
