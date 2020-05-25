 #!/bin/bash 
 date >tests/assets/echo.lst
 for i; do 
    echo "this is argument:" $i >>tests/assets/echo.lst
 done