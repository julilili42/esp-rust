Layers of abstractions (lowest to highest):
- PAC: Lowest level of abstraction. In most cases the PAC is interacted with via the HAL.
- HAL: Sits below the BSP. Builds on top of the PAC. Makes interaction with GPIO pins easy without directly dealing with registers. 
- BSP: Tailored to specific development boards. Combines HAL with board-specific configurations. Provides Interface for onboard components like LEDs, buttons and sensors. No popular BSP specific for ESP32. 
