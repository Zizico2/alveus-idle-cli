<?xml version="1.0" encoding="UTF-8"?>
<tileset version="1.10" tiledversion="1.11.2" name="interiors" tilewidth="32" tileheight="32" tilecount="13" columns="0">
 <grid orientation="orthogonal" width="1" height="1"/>
 <tile id="0">
  <image source="tiles/wood_floor.png" width="32" height="32"/>
 </tile>
 <tile id="1">
  <image source="tiles/wood_floor_alt.png" width="32" height="32"/>
 </tile>
 <tile id="2">
  <image source="tiles/wall.png" width="32" height="32"/>
   <properties>
    <property name="obstacle" type="class" propertytype="alveus_idle_cli::components::Obstacle">
     <properties/>
    </property>
   </properties>
 </tile>
 <tile id="3">
  <image source="tiles/wood_door.png" width="32" height="32"/>
 </tile>
 <tile id="4">
  <image source="tiles/prep_table.png" width="32" height="32"/>
   <properties>
    <property name="obstacle" type="class" propertytype="alveus_idle_cli::components::Obstacle">
     <properties/>
    </property>
   </properties>
 </tile>
 <tile id="5">
  <image source="tiles/fridge.png" width="32" height="32"/>
 </tile>
 <tile id="6">
  <image source="tiles/seed_chest.png" width="32" height="32"/>
 </tile>
 <tile id="7">
  <image source="tiles/sand_floor.png" width="32" height="32"/>
 </tile>
 <tile id="8">
  <image source="tiles/sand_floor_alt.png" width="32" height="32"/>
 </tile>
 <tile id="9">
  <image source="tiles/fence.png" width="32" height="32"/>
   <properties>
    <property name="obstacle" type="class" propertytype="alveus_idle_cli::components::Obstacle">
     <properties/>
    </property>
   </properties>
 </tile>
 <tile id="10">
  <image source="tiles/gate.png" width="32" height="32"/>
 </tile>
 <tile id="11">
  <image source="tiles/shelter.png" width="32" height="32"/>
   <properties>
    <property name="obstacle" type="class" propertytype="alveus_idle_cli::components::Obstacle">
     <properties/>
    </property>
   </properties>
 </tile>
 <tile id="12">
  <image source="tiles/feeding_dish.png" width="32" height="32"/>
 </tile>
</tileset>
