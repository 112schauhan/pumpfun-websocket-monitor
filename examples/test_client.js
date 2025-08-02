const WebSocket = require("ws")

const WS_URL = process.argv[2] || "ws://localhost:8081"

console.log(`Connecting to ${WS_URL}...`)

const ws = new WebSocket(WS_URL)

ws.on("open", function () {
  console.log("✅ Connected to pump.fun monitor")

  // Subscribe to all events
  const subscribeMessage = {
    type: "subscribe",
    filters: {
      min_supply: 1000000,
    },
  }

  ws.send(JSON.stringify(subscribeMessage))
  console.log("📡 Subscribed to token creation events")

  // Send periodic pings
  setInterval(() => {
    if (ws.readyState === WebSocket.OPEN) {
      ws.send(JSON.stringify({ type: "ping" }))
    }
  }, 30000)
})

ws.on("message", function (data) {
  try {
    const message = JSON.parse(data)

    switch (message.type) {
      case "token_created":
        console.log("\n🚀 NEW TOKEN CREATED!")
        console.log(`   Name: ${message.token.name}`)
        console.log(`   Symbol: ${message.token.symbol}`)
        console.log(`   Mint: ${message.token.mint_address}`)
        console.log(`   Creator: ${message.token.creator}`)
        console.log(`   Supply: ${message.token.supply.toLocaleString()}`)
        console.log(`   Decimals: ${message.token.decimals}`)
        console.log(`   TX: ${message.transaction_signature}`)
        console.log(`   Time: ${message.timestamp}`)
        console.log(`   Bonding Curve: ${message.pump_data.bonding_curve}`)
        console.log(
          `   SOL Reserves: ${(
            message.pump_data.virtual_sol_reserves / 1e9
          ).toFixed(2)} SOL`
        )
        console.log(
          `   Token Reserves: ${message.pump_data.virtual_token_reserves.toLocaleString()}`
        )
        break

      case "subscribed":
        console.log(`✅ ${message.message}`)
        break

      case "pong":
        console.log("🏓 Pong received")
        break

      case "error":
        console.error(`❌ Error: ${message.message}`)
        break

      default:
        console.log("📨 Received message:", message)
    }
  } catch (error) {
    console.error("❌ Failed to parse message:", error)
    console.log("Raw data:", data.toString())
  }
})

ws.on("close", function (code, reason) {
  console.log(`❌ Connection closed: ${code} - ${reason}`)
  process.exit(1)
})

ws.on("error", function (error) {
  console.error("❌ WebSocket error:", error)
  process.exit(1)
})

// Handle process signals
process.on("SIGINT", function () {
  console.log("\n👋 Disconnecting...")
  ws.close()
  process.exit(0)
})

process.on("SIGTERM", function () {
  console.log("\n👋 Disconnecting...")
  ws.close()
  process.exit(0)
})

console.log("🎧 Listening for pump.fun token creation events...")
console.log("Press Ctrl+C to exit")
