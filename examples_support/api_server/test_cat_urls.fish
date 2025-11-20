#!/usr/bin/env fish

set urls (cat ./src/cat_urls.txt)
for url in $urls
    echo "testing " $url
    curl -f -I $url
    or return 1
end
