# IED

IED allows you to create very large [zip
bombs](https://en.wikipedia.org/wiki/Zip_bomb), suitable for tearing down
[malicious web scrapers](https://idiallo.com/blog/zipbomb-protection).

## Usage

```
ied [Content-Encoding] [bomb size] (-f [file] | -l [literal] | -L [ASCII code])...
```

## Examples

### Valid HTML file filled with 'a' characters

```
ied 'gzip, gzip' 1048576 -f head.html -l a -f tail.html
```

### Googol byte zip bomb filled with 'A' characters

```
ied $(python3 -c "print((',gzip'*34)[1:])") 1 -L 65
```

### Googol byte zip bomb which is also a valid HTML file

```
ied $(python3 -c "print((',gzip'*34)[1:])") 1 -f head.html -L 65 -f tail.html
```
