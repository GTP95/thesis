#! /bin/bash

irma server -vvvvvv --no-tls --sse --allow-unsigned-callbacks --url http://192.168.66.162:8088 -l 0.0.0.0 --port 8088 --schemes-update 0
