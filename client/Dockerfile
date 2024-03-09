FROM alpine:latest

EXPOSE 8000
WORKDIR /serve

RUN apk --no-cache -U add python3 && \
    apk upgrade --no-cache -U -a  
# Patch OpenSSL vulnerability^

COPY ./dist /serve
COPY ./serve.py /bin/serve.py

ENTRYPOINT [ "python3", "/bin/serve.py" ]
