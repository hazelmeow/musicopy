set windows-shell := ["powershell.exe", "-NoLogo", "-Command"]

default:
  just --list

test:
  cargo nextest run --package musicopy

cov:
  cargo llvm-cov --html nextest --package musicopy

download-icon icon:
  curl "https://fonts.gstatic.com/s/i/short-term/release/materialsymbolsoutlined/{{icon}}/default/24px.xml" -o ./composeApp/src/commonMain/composeResources/drawable/{{icon}}_24px.xml
  sed -i 's/?attr\/colorControlNormal/#FFFFFF/g' ./composeApp/src/commonMain/composeResources/drawable/{{icon}}_24px.xml
  sed -i 's/@android:color\/white/#FFFFFF/g' ./composeApp/src/commonMain/composeResources/drawable/{{icon}}_24px.xml
