This is my personal server which serves stuff behind an Nginx proxy for [my website](https://waffles.space).

In addition to serving static files, it supports:

- Some caching based on mtime and etags
- Serving private paths (autogenerates public links for private paths and rotates them over intervals)
- Sends SMS (through AWS SNS) whenever private paths are accessed
