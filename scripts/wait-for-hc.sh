#!/usr/bin/env bash

until nc -z localhost 8888;
do
  echo "Waiting for Holochain to be available on ws://localhost:8888..."
  sleep 3s
done
