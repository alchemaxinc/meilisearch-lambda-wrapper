import http from "k6/http";
import { check, sleep } from "k6";
import { Counter } from "k6/metrics";
import exec from "k6/execution";

const BASE_URL = __ENV.BASE_URL || "http://localhost:8080";
const MASTER_KEY = __ENV.MEILI_MASTER_KEY || "test-master-key-12345";
const moviesCSV = open("../../wrapper/tests/fixtures/movies.csv");

const SEARCH_TERMS = [
  "action",
  "love",
  "war",
  "space",
  "comedy",
  "hero",
  "dark",
  "night",
  "world",
  "king",
  "dragon",
  "star",
  "city",
  "dream",
  "fire",
];
const DOCS_PER_WRITE_BATCH = 50;

const headers = { Authorization: `Bearer ${MASTER_KEY}` };
const jsonHeaders = { ...headers, "Content-Type": "application/json" };
const csvHeaders = { ...headers, "Content-Type": "text/csv" };

const writeFailures = new Counter("write_failures");
const readFailures = new Counter("read_failures");

export const options = {
  scenarios: {
    concurrent_reads: {
      executor: "constant-vus",
      vus: 10,
      duration: "30s",
      startTime: "0s",
      exec: "searchLoad",
      tags: { scenario: "reads" },
    },
    concurrent_writes_isolated: {
      executor: "constant-vus",
      vus: 10,
      duration: "30s",
      startTime: "35s",
      exec: "writeLoadIsolated",
      tags: { scenario: "writes_isolated" },
    },
    concurrent_writes_shared: {
      executor: "constant-vus",
      vus: 10,
      duration: "30s",
      startTime: "70s",
      exec: "writeLoadShared",
      tags: { scenario: "writes_shared" },
    },
    mixed_workload: {
      executor: "constant-vus",
      vus: 15,
      duration: "60s",
      startTime: "105s",
      exec: "mixedLoad",
      tags: { scenario: "mixed" },
    },
  },

  thresholds: {
    http_req_failed: ["rate<0.01"],
    "http_req_duration{scenario:reads}": ["p(95)<500", "p(99)<1000"],
    "http_req_duration{scenario:writes_isolated}": [
      "p(95)<10000",
      "p(99)<20000",
    ],
    "http_req_duration{scenario:writes_shared}": ["p(95)<15000", "p(99)<30000"],
    "http_req_duration{scenario:mixed}": ["p(95)<10000", "p(99)<20000"],
    write_failures: ["count<15"],
    read_failures: ["count<10"],
  },
};

function generateDocuments(prefix, iteration, count) {
  const docs = [];
  for (let i = 0; i < count; i++) {
    docs.push({
      id: `${prefix}-${iteration}-${i}`,
      title: `Stress Test Document ${prefix} ${iteration} ${i}`,
      overview: `Generated during stress testing. Prefix: ${prefix}, iteration: ${iteration}, doc: ${i}. This provides searchable content for full-text search validation under load.`,
      genres: ["stress-test"],
      release_date: "2026-01-01",
    });
  }
  return JSON.stringify(docs);
}

function randomSearchTerm() {
  return SEARCH_TERMS[Math.floor(Math.random() * SEARCH_TERMS.length)];
}

function tryParseJson(body) {
  try {
    return JSON.parse(body);
  } catch (_) {
    return null;
  }
}

export function setup() {
  console.log("Seeding movies index with full CSV dataset…");

  const res = http.post(
    `${BASE_URL}/indexes/movies/documents?primaryKey=id`,
    moviesCSV,
    { headers: csvHeaders, timeout: "120s" },
  );

  const seedOk = check(res, {
    "seed: status 200": (r) => r.status === 200,
    "seed: task succeeded": (r) => {
      const body = tryParseJson(r.body);
      return body !== null && body.status === "succeeded";
    },
  });

  if (!seedOk) {
    console.error(`Seed failed: ${res.status} — ${res.body}`);
    return { seeded: false };
  }

  const task = JSON.parse(res.body);
  console.log(
    `Seed complete: ${task.details.indexedDocuments} documents indexed`,
  );
  return { seeded: true, indexedDocuments: task.details.indexedDocuments };
}

export function searchLoad() {
  const res = http.get(
    `${BASE_URL}/indexes/movies/search?q=${randomSearchTerm()}&limit=20`,
    { headers, tags: { name: "search" } },
  );

  const ok = check(res, {
    "search: status 200": (r) => r.status === 200,
    "search: has hits": (r) => {
      const body = tryParseJson(r.body);
      return body !== null && body.hits !== undefined;
    },
  });

  if (!ok) readFailures.add(1);
  sleep(0.1);
}

export function writeLoadIsolated() {
  const vuId = exec.vu.idInTest;
  const iter = exec.vu.iterationInScenario;
  const body = generateDocuments(`iso-vu${vuId}`, iter, DOCS_PER_WRITE_BATCH);

  const res = http.post(
    `${BASE_URL}/indexes/stress-isolated-vu${vuId}/documents?primaryKey=id`,
    body,
    { headers: jsonHeaders, timeout: "30s", tags: { name: "write_isolated" } },
  );

  const ok = check(res, {
    "write_isolated: status 200": (r) => r.status === 200,
    "write_isolated: task succeeded": (r) => {
      const body = tryParseJson(r.body);
      return body !== null && body.status === "succeeded";
    },
  });

  if (!ok) writeFailures.add(1);
  sleep(0.5);
}

export function writeLoadShared() {
  const vuId = exec.vu.idInTest;
  const iter = exec.vu.iterationInScenario;
  const body = generateDocuments(
    `shared-vu${vuId}`,
    iter,
    DOCS_PER_WRITE_BATCH,
  );

  const res = http.post(
    `${BASE_URL}/indexes/stress-shared/documents?primaryKey=id`,
    body,
    { headers: jsonHeaders, timeout: "60s", tags: { name: "write_shared" } },
  );

  const ok = check(res, {
    "write_shared: status 200": (r) => r.status === 200,
    "write_shared: task succeeded": (r) => {
      const body = tryParseJson(r.body);
      return body !== null && body.status === "succeeded";
    },
  });

  if (!ok) writeFailures.add(1);
  sleep(0.5);
}

export function mixedLoad() {
  const vuId = exec.vu.idInTest;
  const iter = exec.vu.iterationInScenario;

  if (Math.random() < 0.7) {
    const res = http.get(
      `${BASE_URL}/indexes/movies/search?q=${randomSearchTerm()}&limit=20`,
      { headers, tags: { name: "mixed_search" } },
    );
    if (!check(res, { "mixed_search: status 200": (r) => r.status === 200 }))
      readFailures.add(1);
  } else {
    const body = generateDocuments(
      `mixed-vu${vuId}`,
      iter,
      DOCS_PER_WRITE_BATCH,
    );
    const res = http.post(
      `${BASE_URL}/indexes/stress-mixed/documents?primaryKey=id`,
      body,
      { headers: jsonHeaders, timeout: "30s", tags: { name: "mixed_write" } },
    );
    if (
      !check(res, {
        "mixed_write: status 200": (r) => r.status === 200,
        "mixed_write: task succeeded": (r) => {
          const body = tryParseJson(r.body);
          return body !== null && body.status === "succeeded";
        },
      })
    )
      writeFailures.add(1);
  }

  sleep(0.2);
}

export function teardown(data) {
  if (!data.seeded) {
    console.warn("Skipping teardown — seed failed");
    return;
  }

  console.log("\n=== Teardown: data consistency checks ===\n");

  const moviesRes = http.get(`${BASE_URL}/indexes/movies/stats`, { headers });
  if (moviesRes.status === 200) {
    const stats = JSON.parse(moviesRes.body);
    console.log(`  Movies index:  ${stats.numberOfDocuments} documents`);
    check(moviesRes, {
      "teardown: movies count matches seed": () =>
        stats.numberOfDocuments === data.indexedDocuments,
    });
  }

  const sharedRes = http.get(`${BASE_URL}/indexes/stress-shared/stats`, {
    headers,
  });
  if (sharedRes.status === 200) {
    const stats = JSON.parse(sharedRes.body);
    console.log(`  Shared write index: ${stats.numberOfDocuments} documents`);
  }

  const sharedSearch = http.get(
    `${BASE_URL}/indexes/stress-shared/search?q=stress+test&limit=5`,
    { headers },
  );
  if (sharedSearch.status === 200) {
    const results = JSON.parse(sharedSearch.body);
    console.log(
      `  Shared index sample search: ${results.hits.length} hits returned`,
    );
    check(sharedSearch, {
      "teardown: shared index is searchable": () => results.hits.length > 0,
    });
  }

  const mixedRes = http.get(`${BASE_URL}/indexes/stress-mixed/stats`, {
    headers,
  });
  if (mixedRes.status === 200) {
    const stats = JSON.parse(mixedRes.body);
    console.log(`  Mixed write index: ${stats.numberOfDocuments} documents`);
  }

  const tasksRes = http.get(`${BASE_URL}/tasks?statuses=failed&limit=1`, {
    headers,
  });
  if (tasksRes.status === 200) {
    const tasks = JSON.parse(tasksRes.body);
    console.log(`  Failed tasks:  ${tasks.total}`);
    check(tasksRes, {
      "teardown: no failed tasks": () => tasks.total === 0,
    });
  }

  console.log("\n=== Teardown complete ===\n");
}
