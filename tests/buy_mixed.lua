--example: wrk -t10 -c10 -d8s -s buy_mixed.lua --latency http://localhost:8080
math.randomseed(os.time())
users = { "user1", "user2", "user3" }
-- prices = {3,4,5}  -- Vary for multi-heaps

request = function()
  local user = users[math.random(#users)]
  -- local price = prices[math.random(#prices)]
  path = "/buy"
  method = "POST"
  headers = {}
  headers["Content-Type"] = "application/json"
  body = string.format('{"username":"%s","volume":50,"price":%d}', user, 5)
  return wrk.format(method, path, headers, body)
end
