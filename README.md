# IOT-Craft

Client:

```
brew install mosquitto
```

Watch:
```
mosquitto_sub -h localhost -p 1883 -t home/cube/light -i bevy_subscriber
```

Change:
```
mosquitto_pub -h localhost -p 1883 -t home/cube/light -m "ON"
mosquitto_pub -h localhost -p 1883 -t home/cube/light -m "OFF"
```
