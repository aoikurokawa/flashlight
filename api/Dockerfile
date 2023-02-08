FROM golang:alpine as builder

WORKDIR /build

COPY go.mod ./
COPY go.sum ./

RUN go mod download

COPY *.go ./

RUN go build -o /shorten-url

# stage 2
FROM alpine

RUN adduser -S -D -H -h /app appuser

USER appuser

COPY . /app

COPY --from=builder /build/shorten-url /app/

WORKDIR /app

EXPOSE 3000

CMD [ "/shorten-url" ]
