package main

import (
	"context"
	"errors"
	"io"
	"machine"
	"math/rand"
	"net"
	"time"

	mqtt "github.com/soypat/natiu-mqtt"
	"tinygo.org/x/drivers/netdev"
	nl "tinygo.org/x/drivers/netlink"
	link "tinygo.org/x/espradio/netlink"
)

var (
	ssid     string = "tinygo"
	password string = "gophercamp"
	broker   string = "192.168.1.100:1883"
)

const (
	// MQTT Settings
	MQTT_KEEP_ALIVE = 60 // seconds

	// Joystick Configuration
	JOYSTICK_X_PIN    = machine.ADC4 // X-axis - GPIO4 (ADC1_CH0)
	JOYSTICK_Y_PIN    = machine.ADC6 // Y-axis - GPIO6 (ADC1_CH2)
	ADC_RESOLUTION    = 65520
	JOYSTICK_CENTER   = 32760
	JOYSTICK_DEADZONE = 10000

	// Movement Settings
	MOVEMENT_SPEED    = 0.2     // World units per update
	UPDATE_RATE       = 50      // Joystick read rate (milliseconds)
	POSE_UPDATE_RATE  = 100     // MQTT publish rate (milliseconds)
)

// JoystickInput manages joystick state and world position
type JoystickInput struct {
	xPin *machine.ADC
	yPin *machine.ADC
	posX float32 // World X position
	posY float32 // World Y position
	posZ float32 // World Z position (constant)
}

func main() {
	// Initialize serial
	serial := machine.Serial
	serial.Configure(machine.UARTConfig{BaudRate: 115200})
	serial.Write([]byte("ESP32-C3 IoTCraft Player\r\n"))
	serial.Write([]byte("==========================\r\n\r\n"))

	time.Sleep(2 * time.Second)

	// Initialize joystick
	joystick := &JoystickInput{}
	joystick.Init()

	// Connect to WiFi
	if err := connectToWiFi(); err != nil {
		failure("WiFi connection failed: " + err.Error())
	}

	// Generate unique player ID
	playerId := "esp32c3-player-" + randomString(6)
	serial.Write([]byte("Player ID: "))
	serial.Write([]byte(playerId))
	serial.Write([]byte("\r\n"))

	// Connect to MQTT
	conn, client, err := connectMQTT(playerId)
	if err != nil {
		failure("MQTT connection failed: " + err.Error())
	}
	defer conn.Close()

	// Send device announcement
	sendDeviceAnnouncement(client, playerId)
	serial.Write([]byte("Device announcement sent\r\n"))

	serial.Write([]byte("\r\nStarting game loop...\r\n"))
	serial.Write([]byte("Use joystick to move in 3D world\r\n"))
	serial.Write([]byte("Desktop client will spawn your avatar\r\n\r\n"))

	// Game loop
	ticker := time.NewTicker(UPDATE_RATE * time.Millisecond)
	lastPoseTime := time.Now()

	for {
		<-ticker.C

		// Read joystick
		dirX, dirY := joystick.Read()

		// Update position
		joystick.Update(dirX, dirY)

		// Send pose message every 100ms (10 Hz)
		if time.Since(lastPoseTime) > time.Millisecond*POSE_UPDATE_RATE {
			sendPoseMessage(client, playerId, joystick)
			lastPoseTime = time.Now()

			// Debug output
			serial.Write([]byte("Position: X="))
			printFloat(serial, joystick.posX)
			serial.Write([]byte(" Y="))
			printFloat(serial, joystick.posY)
			serial.Write([]byte(" Z="))
			printFloat(serial, joystick.posZ)
			serial.Write([]byte("\r\n"))
		}

		// Handle incoming MQTT messages
		conn.SetReadDeadline(time.Now().Add(10*time.Millisecond))
		client.HandleNext()
	}
}

// Init initializes joystick and starting position
func (j *JoystickInput) Init() {
	serial := machine.Serial
	serial.Write([]byte("Initializing joystick...\r\n"))

	machine.InitADC()
	j.xPin = &machine.ADC{Pin: JOYSTICK_X_PIN}
	j.yPin = &machine.ADC{Pin: JOYSTICK_Y_PIN}
	j.xPin.Configure(machine.ADCConfig{})
	j.yPin.Configure(machine.ADCConfig{})

	// Start at origin (above ground)
	j.posX = 0.0
	j.posY = 20.0 // Start above ground level
	j.posZ = 0.0

	// Allow ADC to stabilize
	time.Sleep(time.Millisecond * 100)
	serial.Write([]byte("Joystick ready\r\n"))
}

// Read returns normalized joystick direction (-1.0 to 1.0)
func (j *JoystickInput) Read() (dirX, dirY float32) {
	rawX := readADC(j.xPin)
	rawY := readADC(j.yPin)

	// Apply deadzone
	if rawX > JOYSTICK_CENTER-JOYSTICK_DEADZONE &&
		rawX < JOYSTICK_CENTER+JOYSTICK_DEADZONE {
		rawX = JOYSTICK_CENTER
	}
	if rawY > JOYSTICK_CENTER-JOYSTICK_DEADZONE &&
		rawY < JOYSTICK_CENTER+JOYSTICK_DEADZONE {
		rawY = JOYSTICK_CENTER
	}

	// Convert to direction (-1.0 to 1.0)
	dirX = (float32(rawX) - float32(JOYSTICK_CENTER)) / float32(JOYSTICK_CENTER)
	dirY = (float32(rawY) - float32(JOYSTICK_CENTER)) / float32(JOYSTICK_CENTER)

	// Clamp to valid range
	if dirX < -1.0 {
		dirX = -1.0
	} else if dirX > 1.0 {
		dirX = 1.0
	}
	if dirY < -1.0 {
		dirY = -1.0
	} else if dirY > 1.0 {
		dirY = 1.0
	}

	return dirX, dirY
}

// Update position based on joystick direction
func (j *JoystickInput) Update(dirX, dirY float32) {
	j.posX += dirX * MOVEMENT_SPEED
	j.posY += dirY * MOVEMENT_SPEED
	// Z stays constant (no vertical movement from joystick)
}

// readADC performs oversampling for stable readings
func readADC(adc *machine.ADC) uint32 {
	const samples = 5
	var sum uint32

	for i := 0; i < samples; i++ {
		sum += uint32(adc.Get())
		time.Sleep(time.Microsecond * 100)
	}

	return sum / samples
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
		OnPub: func(_ mqtt.Header, _ mqtt.VariablesPublish, r io.Reader) error {
			// We don't expect incoming messages, but handle them anyway
			message, _ := io.ReadAll(r)
			serial.Write([]byte("Received: "))
			serial.Write([]byte(string(message)))
			serial.Write([]byte("\r\n"))
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

func sendDeviceAnnouncement(client *mqtt.Client, playerId string) {
	announcement := `{"device_id":"` + playerId + `","device_type":"player","device_name":"ESP32-C3 Game Controller","state":"online","location":{"x":0,"y":20,"z":0}}`

	pubFlags, _ := mqtt.NewPublishFlags(mqtt.QoS0, false, false)
	pubVar := mqtt.VariablesPublish{
		TopicName: []byte("devices/announce"),
	}
	client.PublishPayload(pubFlags, pubVar, []byte(announcement))
}

func sendPoseMessage(client *mqtt.Client, playerId string, j *JoystickInput) {
	// Get timestamp (milliseconds since epoch)
	timestamp := uint32(time.Now().Unix() * 1000)

	// Build JSON pose message
	poseMsg := `{"player_id":"` + playerId + `","player_name":"ESP32-C3 Player","pos":[` +
		floatToString(j.posX) + `,` +
		floatToString(j.posY) + `,` +
		floatToString(j.posZ) + `],"yaw":0.0,"pitch":0.0,"ts":` +
		uint32ToString(timestamp) + `}`

	topic := "player/" + playerId + "/pose"
	pubFlags, _ := mqtt.NewPublishFlags(mqtt.QoS0, false, false)
	pubVar := mqtt.VariablesPublish{
		TopicName: []byte(topic),
	}
	client.PublishPayload(pubFlags, pubVar, []byte(poseMsg))
}

// Helper functions for JSON serialization

func floatToString(f float32) string {
	// Simple float to string conversion with 2 decimal places
	intPart := int32(f)
	fracPart := int32((f - float32(intPart)) * 100)

	if fracPart < 0 {
		fracPart = -fracPart
	}

	return int32ToString(intPart) + "." + int32ToString(fracPart)
}

func int32ToString(n int32) string {
	if n == 0 {
		return "0"
	}

	var buf [20]byte
	i := 20
	neg := n < 0
	if neg {
		n = -n
	}

	for n > 0 && i > 0 {
		i--
		buf[i] = byte('0' + n%10)
		n /= 10
	}

	if neg {
		i--
		buf[i] = '-'
	}

	return string(buf[i:])
}

func uint32ToString(n uint32) string {
	if n == 0 {
		return "0"
	}

	var buf [20]byte
	i := 20
	for n > 0 && i > 0 {
		i--
		buf[i] = byte('0' + n%10)
		n /= 10
	}

	return string(buf[i:])
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

func printFloat(serial machine.Serialer, f float32) {
	neg := f < 0
	if neg {
		f = -f
		serial.WriteByte('-')
	}

	intPart := int32(f)
	fracPart := int32((f - float32(intPart)) * 100)

	printInt(serial, uint32(intPart))
	serial.WriteByte('.')
	if fracPart < 10 {
		serial.WriteByte('0')
	}
	printInt(serial, uint32(fracPart))
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
