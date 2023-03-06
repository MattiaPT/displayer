# Displayer

An imaging tool aimed to visualize location data contained in images.

## Setup
clone this repository and get your google maps api key, then add this line to ~/.bashrc
```
export GOOGLE_MAPS_API_KEY=<your_api_key>
```

## Usage
```
$ cd displayer
$ cargo run -- --port <PORT> --data <DIRECTORY_PATH>
```
Meaning of flags: <br/>
-> hosting on http://localhost:PORT <br/>
-> recursive search for images in DIRECTORY_PATH

## Preview
![image](https://user-images.githubusercontent.com/52134864/222930364-57070b10-b1c6-4b79-ad51-96030b7596d9.png)
