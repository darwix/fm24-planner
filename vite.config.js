export default {
  test: { include: ["src/**/*.test.js"] },
  server: { proxy: { "/api": "http://127.0.0.1:4242" } },
};
