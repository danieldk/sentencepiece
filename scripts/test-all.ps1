$ErrorActionPreference = "Stop"

$cache_dir = "sentencepiece/testdata"
$test_dir = "testdata"

$models = @{
    "ALBERT_BASE_MODEL" = "https://s3.amazonaws.com/models.huggingface.co/bert/albert-base-v1-spiece.model"
}

if (-not (Test-Path $cache_dir)) {
    New-Item -ItemType Directory -Path $cache_dir | Out-Null
}

foreach ($var in $models.GetEnumerator()) {
    $url = $var.Value
    $bn = [System.IO.Path]::GetFileName($url)
    $data = Join-Path $cache_dir $bn

    if (-not (Test-Path $data)) {
        Invoke-WebRequest -OutFile "$data" -Uri "$url"
    }
    Set-Item -Path "Env:$($var.Key)" -Value "$test_dir/$bn"
}

cargo test --features albert-tests
