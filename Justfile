set windows-shell := ["powershell.exe", "-NoLogo", "-Command"]

default:
  just --list

tailwind:
  npx @tailwindcss/cli -i ./tailwind.css -o ./assets/tailwind.css --watch
