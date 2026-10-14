param(
    [Parameter(Mandatory = $true)][string]$Folder,
    [string]$OnlyFile
)
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8

Add-Type -AssemblyName System.Runtime.WindowsRuntime
Add-Type -AssemblyName System.Drawing

$null = [Windows.Storage.StorageFile, Windows.Storage, ContentType = WindowsRuntime]
$null = [Windows.Storage.FileAccessMode, Windows.Storage, ContentType = WindowsRuntime]
$null = [Windows.Graphics.Imaging.BitmapDecoder, Windows.Graphics.Imaging, ContentType = WindowsRuntime]
$null = [Windows.Media.Ocr.OcrEngine, Windows.Media.Ocr, ContentType = WindowsRuntime]

function Wait-WinRt($Operation, [Type]$ResultType) {
    $method = [System.WindowsRuntimeSystemExtensions].GetMethods() |
        Where-Object { $_.Name -eq 'AsTask' -and $_.IsGenericMethodDefinition -and $_.GetParameters().Count -eq 1 } |
        Select-Object -First 1
    $task = $method.MakeGenericMethod($ResultType).Invoke($null, @($Operation))
    return $task.GetAwaiter().GetResult()
}

function Read-OcrCrop($Source, [int]$X, [int]$Y, [int]$Width, [int]$Height) {
    $temporary = Join-Path ([IO.Path]::GetTempPath()) ([guid]::NewGuid().ToString() + '.png')
    $crop = $null
    $graphics = $null
    $stream = $null
    $bitmap = $null
    try {
        $crop = [Drawing.Bitmap]::new($Width * 4, $Height * 4)
        $graphics = [Drawing.Graphics]::FromImage($crop)
        $graphics.InterpolationMode = [Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
        if ($Y -eq 263) {
            $matrix = [Drawing.Imaging.ColorMatrix]::new()
            $matrix.Matrix00 = -1; $matrix.Matrix11 = -1; $matrix.Matrix22 = -1
            $matrix.Matrix40 = 1; $matrix.Matrix41 = 1; $matrix.Matrix42 = 1; $matrix.Matrix44 = 1
            $attributes = [Drawing.Imaging.ImageAttributes]::new()
            $attributes.SetColorMatrix($matrix)
            $graphics.DrawImage($Source, [Drawing.Rectangle]::new(0, 0, $Width * 4, $Height * 4), $X, $Y, $Width, $Height, [Drawing.GraphicsUnit]::Pixel, $attributes)
            $attributes.Dispose()
        }
        else {
            $graphics.DrawImage($Source, [Drawing.Rectangle]::new(0, 0, $Width * 4, $Height * 4), $X, $Y, $Width, $Height, [Drawing.GraphicsUnit]::Pixel)
        }
        $crop.Save($temporary, [Drawing.Imaging.ImageFormat]::Png)
        $storageFile = Wait-WinRt ([Windows.Storage.StorageFile]::GetFileFromPathAsync($temporary)) ([Windows.Storage.StorageFile])
        $stream = Wait-WinRt ($storageFile.OpenAsync([Windows.Storage.FileAccessMode]::Read)) ([Windows.Storage.Streams.IRandomAccessStream])
        $decoder = Wait-WinRt ([Windows.Graphics.Imaging.BitmapDecoder]::CreateAsync($stream)) ([Windows.Graphics.Imaging.BitmapDecoder])
        $bitmap = Wait-WinRt ($decoder.GetSoftwareBitmapAsync()) ([Windows.Graphics.Imaging.SoftwareBitmap])
        $recognized = Wait-WinRt ($engine.RecognizeAsync($bitmap)) ([Windows.Media.Ocr.OcrResult])
        return $recognized.Text.Trim()
    }
    finally {
        if ($stream) { $stream.Dispose() }
        if ($bitmap) { $bitmap.Dispose() }
        if ($graphics) { $graphics.Dispose() }
        if ($crop) { $crop.Dispose() }
        Remove-Item -LiteralPath $temporary -Force -ErrorAction SilentlyContinue
    }
}

$engine = [Windows.Media.Ocr.OcrEngine]::TryCreateFromUserProfileLanguages()
if ($null -eq $engine) { throw 'Windows OCR has no installed recognition language.' }
$results = [System.Collections.Generic.List[object]]::new()
$files = Get-ChildItem -LiteralPath $Folder -Filter '*.png' -File | Sort-Object Name
if ($OnlyFile) { $files = @($files | Where-Object { $_.Name -eq $OnlyFile }) }
foreach ($file in $files) {
    $source = $null
    try {
        $source = [Drawing.Image]::FromFile($file.FullName)
        $igt = Read-OcrCrop $source 45 263 195 28
        $coordinates = Read-OcrCrop $source 45 291 300 28
        $results.Add([pscustomobject]@{ file = $file.Name; text = "$igt`n$coordinates"; error = $null })
    }
    catch {
        $results.Add([pscustomobject]@{ file = $file.Name; text = $null; error = $_.Exception.Message })
    }
    finally {
        if ($source) { $source.Dispose() }
    }
}
$results | ConvertTo-Json -Depth 3 -Compress
