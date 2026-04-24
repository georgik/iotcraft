package main

import (
	"context"
	"errors"
	"fmt"
	"image/color"
	"io"
	"machine"
	"math/rand"
	"net"
	"time"

	mqtt "github.com/soypat/natiu-mqtt"
	"tinygo.org/x/drivers/netdev"
	nl "tinygo.org/x/drivers/netlink"
	link "tinygo.org/x/espradio/netlink"
	"tinygo.org/x/drivers/ws2812"
)

var (
	ssid     string = "tinygo"
	password string = "gophercamp"
	broker   string = "192.168.4.1:1883"
)

const (
	// MQTT Settings
	MQTT_KEEP_ALIVE = 60 // seconds
	TOPIC_PREFIX    = "home/"
	TOPIC_SUFFIX    = "/light"

	// LED
	LED_PIN        = machine.GPIO8
	LED_BRIGHTNESS = 51 // 20%
)

var (
	ledDevice ws2812.Device
	ledState  bool = false
)

func main() {
	// Initialize serial
	serial := machine.Serial
	serial.Configure(machine.UARTConfig{BaudRate: 115200})
	serial.Write([]byte("WiFi MQTT Lamp - ESP32-C3\r\n"))
	serial.Write([]byte("========================\r\n\r\n"))

	time.Sleep(2 * time.Second)

	// Setup LED
	ledDevice = setupLED()
	setLED(false)

	// Connect to WiFi
	if err := connectToWiFi(); err != nil {
		failure("WiFi connection failed: " + err.Error())
	}

	// Generate unique client ID
	clientId := "esp32c3-lamp-" + randomString(6)
	serial.Write([]byte("Client ID: "))
	serial.Write([]byte(clientId))
	serial.Write([]byte("\r\n"))

	// Connect to MQTT broker
	conn, client, err := connectMQTT(clientId)
	if err != nil {
		failure("MQTT connection failed: " + err.Error())
	}
	defer conn.Close()

	// Subscribe to light control topic
	topic := TOPIC_PREFIX + clientId + TOPIC_SUFFIX
	ctx, _ := context.WithTimeout(context.Background(), 10*time.Second)
	err = client.Subscribe(ctx, mqtt.VariablesSubscribe{
		PacketIdentifier: 1,
		TopicFilters: []mqtt.SubscribeRequest{
			{TopicFilter: []byte(topic), QoS: mqtt.QoS0},
		},
	})

	if err != nil {
		failure("Subscribe failed: " + err.Error())
	}

	serial.Write([]byte("Subscribed to topic: "))
	serial.Write([]byte(topic))
	serial.Write([]byte("\r\n"))

	// Send device announcement
	announcement := fmt.Sprintf("{\"device_id\":\"%s\",\"device_type\":\"lamp\",\"state\":\"online\"}", clientId)
	pubFlags, _ := mqtt.NewPublishFlags(mqtt.QoS0, false, false)
	pubVar := mqtt.VariablesPublish{
		TopicName: []byte("devices/announce"),
	}
	client.PublishPayload(pubFlags, pubVar, []byte(announcement))
	serial.Write([]byte("Device announcement sent\r\n"))

	// Message processing loop
	serial.Write([]byte("\r\nListening for commands...\r\n"))
	serial.Write([]byte("Send: mosquitto_pub -h 192.168.1.100 -t "))
	serial.Write([]byte(topic))
	serial.Write([]byte(" -m \"ON\"\r\n\r\n"))

	for {
		if !client.IsConnected() {
			failure("MQTT client disconnected")
		}

		conn.SetReadDeadline(time.Now().Add(10 * time.Second))
		err := client.HandleNext()
		if err != nil {
			serial.Write([]byte("Error: "))
			serial.Write([]byte(err.Error()))
			serial.Write([]byte("\r\n"))
			time.Sleep(time.Second)
			continue
		}
	}
}

func setupLED() ws2812.Device {
	led := LED_PIN
	led.Configure(machine.PinConfig{Mode: machine.PinOutput})
	return ws2812.New(led)
}

func setLED(on bool) {
	ledState = on
	var c color.RGBA
	if on {
		c = color.RGBA{R: 255, G: 255, B: 0, A: 255} // Yellow
	} else {
		c = color.RGBA{R: 0, G: 0, B: 0, A: 0} // Off
	}

	// Apply brightness scaling
	scaled := color.RGBA{
		R: uint8(uint16(c.R) * uint16(LED_BRIGHTNESS) / 255),
		G: uint8(uint16(c.G) * uint16(LED_BRIGHTNESS) / 255),
		B: uint8(uint16(c.B) * uint16(LED_BRIGHTNESS) / 255),
		A: 255,
	}

	ledDevice.WriteColors([]color.RGBA{scaled})
}

func connectToWiFi() error {
	serial := machine.Serial
	serial.Write([]byte("Connecting to WiFi...\r\n"))

	radioLink := link.Esplink{}
	netdev.UseNetdev(&radioLink)

	for range 3 {
		err := radioLink.NetConnect(&nl.ConnectParams{
			Ssid:       ssid,
			Passphrase: password,
		})
		if err == nil {
			serial.Write([]byte("WiFi connected!\r\n"))

			// Get IP address
			addr, err := radioLink.Addr()
			if err == nil {
				serial.Write([]byte("IP Address: "))
				serial.Write([]byte(addr.String()))
				serial.Write([]byte("\r\n"))
			}

			return nil
		}

		serial.Write([]byte("Connection failed: "))
		serial.Write([]byte(err.Error()))
		serial.Write([]byte("\r\n"))
		time.Sleep(5 * time.Second)
	}

	return errors.New("failed to connect to WiFi after 3 attempts")
}

func connectMQTT(clientId string) (net.Conn, *mqtt.Client, error) {
	serial := machine.Serial
	serial.Write([]byte("Connecting to MQTT broker at "))
	serial.Write([]byte(broker))
	serial.Write([]byte("...\r\n"))

	// TCP connection with retry
	var conn net.Conn
	for attempt := 0; attempt < 5; attempt++ {
		var err error
		conn, err = net.Dial("tcp", broker)
		if err == nil {
			serial.Write([]byte("TCP connected to "))
			serial.Write([]byte(conn.RemoteAddr().String()))
			serial.Write([]byte("\r\n"))
			break
		}

		serial.Write([]byte("Dial attempt "))
		printInt(serial, uint32(attempt+1))
		serial.Write([]byte(" failed\r\n"))
		time.Sleep(2 * time.Second)
	}

	if conn == nil {
		return nil, nil, errors.New("all TCP connection attempts failed")
	}

	// Create MQTT client
	client := mqtt.NewClient(mqtt.ClientConfig{
		Decoder: mqtt.DecoderNoAlloc{make([]byte, 1500)},
		OnPub: func(header mqtt.Header, vars mqtt.VariablesPublish, r io.Reader) error {
			topic := string(vars.TopicName)
			message, _ := io.ReadAll(r)

			serial.Write([]byte("Received: "))
			serial.Write([]byte(string(message)))
			serial.Write([]byte(" on topic "))
			serial.Write([]byte(topic))
			serial.Write([]byte("\r\n"))

			// Check if it's a light control message
			payload := string(message)
			if payload == "ON" {
				setLED(true)
				serial.Write([]byte("LED turned ON\r\n"))
			} else if payload == "OFF" {
				setLED(false)
				serial.Write([]byte("LED turned OFF\r\n"))
			}

			return nil
		},
	})

	// Connect to broker
	var varconn mqtt.VariablesConnect
	varconn.SetDefaultMQTT([]byte(clientId))
	varconn.KeepAlive = MQTT_KEEP_ALIVE

	ctx, _ := context.WithTimeout(context.Background(), 10*time.Second)
	err := client.Connect(ctx, conn, &varconn)
	if err != nil {
		return nil, nil, err
	}

	serial.Write([]byte("MQTT connected\r\n"))
	return conn, client, nil
}

func randomInt(min, max int) int {
	return min + rand.Intn(max-min)
}

func randomString(length int) string {
	bytes := make([]byte, length)
	for i := 0; i < length; i++ {
		bytes[i] = byte(randomInt(65, 90)) // A-Z
	}
	return string(bytes)
}

func printInt(serial machine.Serialer, n uint32) {
	if n == 0 {
		serial.WriteByte('0')
		return
	}

	var buf [10]byte
	i := 10
	for n > 0 && i > 0 {
		i--
		buf[i] = byte('0' + n%10)
		n /= 10
	}

	for i < 10 {
		serial.WriteByte(buf[i])
		i++
	}
}

func failure(msg string) {
	serial := machine.Serial
	for {
		serial.Write([]byte("FAILURE: "))
		serial.Write([]byte(msg))
		serial.Write([]byte("\r\n"))
		time.Sleep(time.Second)
	}
}
