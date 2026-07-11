"use strict";

const path = require("node:path");
const grpc = require("@grpc/grpc-js");
const protoLoader = require("@grpc/proto-loader");
const { resolveDaemonSocket } = require("./resolveSocket.cjs");

const PROTO_PATH = path.resolve(__dirname, "../../../../proto/rex/v1/rex.proto");

let packageDef = null;

function loadPackage() {
  if (packageDef) return packageDef;
  const definition = protoLoader.loadSync(PROTO_PATH, {
    keepCase: true,
    longs: String,
    enums: String,
    defaults: true,
    oneofs: true,
    includeDirs: [path.resolve(__dirname, "../../../../proto")],
  });
  packageDef = grpc.loadPackageDefinition(definition);
  return packageDef;
}

function createRexClient(socketPath = resolveDaemonSocket()) {
  const pkg = loadPackage();
  const RexService = pkg.rex.v1.RexService;
  return new RexService(
    `unix://${socketPath}`,
    grpc.credentials.createInsecure(),
    {
      "grpc.keepalive_time_ms": 10_000,
      "grpc.keepalive_timeout_ms": 5_000,
    },
  );
}

function unary(client, method, request, metadata = new grpc.Metadata()) {
  return new Promise((resolve, reject) => {
    client[method](request, metadata, (err, response) => {
      if (err) reject(err);
      else resolve(response);
    });
  });
}

module.exports = {
  createRexClient,
  unary,
  grpc,
};
