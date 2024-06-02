# Throws an error if arguments are passed, since none are expected.
[CmdletBinding()]
Param()

$ProjectRoot = (Get-Item $PSScriptRoot).Parent.FullName

docker run --rm -it `
  -v "${ProjectRoot}:/KidneyOS" `
  -w /KidneyOS `
  ghcr.io/kidneyos/kidneyos-builder:latest
