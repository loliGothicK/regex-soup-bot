FROM phusion/baseimage:focal-1.1.0

COPY / /work/
COPY /regexsoup.sh /etc/service/regexsoup/run

RUN chmod +x /work/copy-platform-artifact.sh
RUN chmod +x /etc/service/regexsoup/run

RUN /work/copy-platform-artifact.sh
RUN chmod +x /usr/local/bin/regexsoup
