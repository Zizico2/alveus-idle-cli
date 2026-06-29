<?xml version="1.0" encoding="UTF-8"?>
<tileset version="1.10" tiledversion="1.11.2" name="Tiles" tilewidth="32" tileheight="32" tilecount="4" columns="0">
 <grid orientation="orthogonal" width="1" height="1"/>
 <tile id="0">
  <image source="sand_tile.png" width="32" height="32"/>
 </tile>
 <tile id="1">
  <image source="sand_grass_tile.png" width="32" height="32"/>
 </tile>
 <tile id="2">
  <image source="grass_tile.png" width="32" height="32"/>
 </tile>
 <tile id="3">
  <image source="compost_bin.png" width="32" height="32"/>
   <properties>
    <property name="obstacle" type="class" propertytype="alveus_idle_cli::components::Obstacle">
     <properties/>
    </property>
    <property name="room_object_id" type="class" propertytype="alveus_idle_cli::content::RoomObjectId">
     <properties>
      <property name=":variant" propertytype="alveus_idle_cli::content::RoomObjectId:::Variant" value="CompostBin"/>
     </properties>
    </property>
    <property name="poop_dump" type="class" propertytype="alveus_idle_cli::cleaning::PoopDump">
     <properties>
      <property name="prompt" type="string" value="Empty wheelbarrow"/>
     </properties>
    </property>   </properties>
 </tile>
</tileset>
