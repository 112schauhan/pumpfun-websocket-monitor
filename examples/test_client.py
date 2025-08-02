import asyncio
import json
import sys
import signal
import websockets
from datetime import datetime

class PumpFunClient:
    def __init__(self, url="ws://localhost:8081"):
        self.url = url
        self.websocket = None
        self.running = False
    
    async def connect(self):
        """Connect to the WebSocket server"""
        try:
            print(f"Connecting to {self.url}...")
            self.websocket = await websockets.connect(self.url)
            print("✅ Connected to pump.fun monitor")
            self.running = True
            return True
        except Exception as e:
            print(f"❌ Failed to connect: {e}")
            return False
    
    async def subscribe(self, filters=None):
        """Subscribe to token creation events"""
        if not self.websocket:
            return False
        
        subscribe_message = {
            "type": "subscribe",
            "filters": filters or {"min_supply": 1000000}
        }
        
        try:
            await self.websocket.send(json.dumps(subscribe_message))
            print("📡 Subscribed to token creation events")
            return True
        except Exception as e:
            print(f"❌ Failed to subscribe: {e}")
            return False
    
    async def listen(self):
        """Listen for incoming messages"""
        if not self.websocket:
            return
        
        try:
            async for message in self.websocket:
                await self.handle_message(message)
        except websockets.exceptions.ConnectionClosed:
            print("❌ Connection closed by server")
        except Exception as e:
            print(f"❌ Error listening for messages: {e}")
    
    async def handle_message(self, message):
        """Handle incoming WebSocket message"""
        try:
            data = json.loads(message)
            message_type = data.get("type")
            
            if message_type == "token_created":
                await self.handle_token_created(data)
            elif message_type == "subscribed":
                print(f"✅ {data.get('message', 'Subscribed')}")
            elif message_type == "pong":
                print("🏓 Pong received")
            elif message_type == "error":
                print(f"❌ Error: {data.get('message', 'Unknown error')}")
            else:
                print(f"📨 Received message: {data}")
        
        except json.JSONDecodeError as e:
            print(f"❌ Failed to parse message: {e}")
            print(f"Raw data: {message}")
        except Exception as e:
            print(f"❌ Error handling message: {e}")
    
    async def handle_token_created(self, data):
        """Handle token creation event"""
        token = data.get("token", {})
        pump_data = data.get("pump_data", {})
        
        print("\n🚀 NEW TOKEN CREATED!")
        print(f"   Name: {token.get('name', 'Unknown')}")
        print(f"   Symbol: {token.get('symbol', 'Unknown')}")
        print(f"   Mint: {token.get('mint_address', 'Unknown')}")
        print(f"   Creator: {token.get('creator', 'Unknown')}")
        print(f"   Supply: {token.get('supply', 0):,}")
        print(f"   Decimals: {token.get('decimals', 0)}")
        print(f"   TX: {data.get('transaction_signature', 'Unknown')}")
        print(f"   Time: {data.get('timestamp', 'Unknown')}")
        print(f"   Bonding Curve: {pump_data.get('bonding_curve', 'Unknown')}")
        
        # Convert lamports to SOL
        sol_reserves = pump_data.get('virtual_sol_reserves', 0) / 1e9
        print(f"   SOL Reserves: {sol_reserves:.2f} SOL")
        print(f"   Token Reserves: {pump_data.get('virtual_token_reserves', 0):,}")
    
    async def ping(self):
        """Send periodic ping messages"""
        while self.running and self.websocket:
            try:
                await asyncio.sleep(30)
                if self.websocket and not self.websocket.closed:
                    await self.websocket.send(json.dumps({"type": "ping"}))
            except Exception as e:
                print(f"❌ Ping error: {e}")
                break
    
    async def close(self):
        """Close the WebSocket connection"""
        self.running = False
        if self.websocket:
            await self.websocket.close()
            print("👋 Disconnected")

async def main():
    # Get WebSocket URL from command line or use default
    url = sys.argv[1] if len(sys.argv) > 1 else "ws://localhost:8081"
    
    client = PumpFunClient(url)
    
    # Setup signal handlers for graceful shutdown
    def signal_handler():
        print("\n👋 Shutting down...")
        asyncio.create_task(client.close())
    
    # Handle Ctrl+C
    if sys.platform != "win32":
        loop = asyncio.get_event_loop()
        for sig in (signal.SIGTERM, signal.SIGINT):
            loop.add_signal_handler(sig, signal_handler)
    
    try:
        # Connect to the server
        if not await client.connect():
            return
        
        # Subscribe to events
        if not await client.subscribe():
            return
        
        print("🎧 Listening for pump.fun token creation events...")
        print("Press Ctrl+C to exit")
        
        # Start ping task
        ping_task = asyncio.create_task(client.ping())
        
        # Start listening for messages
        listen_task = asyncio.create_task(client.listen())
        
        # Wait for either task to complete
        done, pending = await asyncio.wait(
            [ping_task, listen_task],
            return_when=asyncio.FIRST_COMPLETED
        )
        
        # Cancel remaining tasks
        for task in pending:
            task.cancel()
            try:
                await task
            except asyncio.CancelledError:
                pass
    
    except KeyboardInterrupt:
        print("\n👋 Interrupted by user")
    except Exception as e:
        print(f"❌ Unexpected error: {e}")
    finally:
        await client.close()

if __name__ == "__main__":
    # Install websockets if not available
    try:
        import websockets
    except ImportError:
        print("❌ websockets library not found. Install it with:")
        print("   pip install websockets")
        sys.exit(1)
    
    asyncio.run(main())