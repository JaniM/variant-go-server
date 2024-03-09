FROM alpine:latest

EXPOSE 8000
WORKDIR /serve

RUN apk --no-cache -U add python3 && \
    apk upgrade --no-cache -U -a  
# Patch OpenSSL vulnerability^

RUN addgroup -S servergrp && \
    adduser -S server -G servergrp && \
    chown -R server:servergrp /serve

USER server

COPY ./dist /serve
COPY ./serve.py /bin/serve.py

ENTRYPOINT [ "python3", "/bin/serve.py" ]
