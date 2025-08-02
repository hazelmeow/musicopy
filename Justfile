set windows-shell := ["powershell.exe", "-NoLogo", "-Command"]

default:
  just --list

test:
  cargo nextest run --package musicopy

cov:
  cargo llvm-cov --html nextest --package musicopy
