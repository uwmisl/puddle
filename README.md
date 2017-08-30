# puddle
PurpleDrop Language (PDL)

## Developing

To run the server, do the following the project root:
```shell
FLASK_APP=puddle/server.py FLASK_DEBUG=1 flask run
```

The `FLASK_DEBUG` will put Flask in the [debug mode], making it give nice
backtraces and automatically restart when files are changed.

[debug mode]: http://flask.pocoo.org/docs/0.12/api/#flask.send_from_directory
