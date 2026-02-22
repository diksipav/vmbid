-- example: for i in {1..3}; do wrk -t1 -c1 -d2s -s sell.lua --latency http://localhost:8080; sleep 3; done
request = function()
  path = "/sell"
  method = "POST"
  headers = {}
  headers["Content-Type"] = "application/json"
  body = '{"volume":500}' -- Matches ~4 bids (50v each)
  return wrk.format(method, path, headers, body)
end
