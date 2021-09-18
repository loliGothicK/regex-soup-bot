FROM phusion/baseimage:focal-1.1.0

COPY / /work/
RUN /work/copy-platform-artifact.sh
COPY /regexsoup.sh /etc/service/regexsoup/run
RUN chmod +x /etc/service/regexsoup/run
