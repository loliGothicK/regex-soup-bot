FROM phusion/baseimage:focal-1.1.0

COPY /regexsoup-binary /usr/local/bin/regexsoup
COPY /regexsoup.sh /etc/service/regexsoup/run

RUN mkdir /etc/service/regexsoup
RUN chmod +x /etc/service/regexsoup/run
