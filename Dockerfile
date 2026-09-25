FROM debian:bookworm-slim
WORKDIR /app

COPY target/release/feed-rs ./feed-rs
COPY public ./public
COPY .env.docker ./.env.docker

EXPOSE 8080
RUN chmod +x ./feed-rs
ENTRYPOINT ["./feed-rs"]
CMD ["--env-file", ".env.docker", "serve"]
