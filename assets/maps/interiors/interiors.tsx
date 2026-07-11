<?xml version="1.0" encoding="UTF-8"?>
<tileset version="1.10" tiledversion="1.11.2" name="interiors" tilewidth="32" tileheight="32" tilecount="18" columns="0">
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
    <property name="obstacle" type="class" propertytype="alveus_components::Obstacle">
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
    <property name="obstacle" type="class" propertytype="alveus_components::Obstacle">
     <properties/>
    </property>
   </properties>
 </tile>
 <tile id="5">
  <image source="tiles/fridge.png" width="32" height="32"/>
   <properties>
    <property name="obstacle" type="class" propertytype="alveus_components::Obstacle">
     <properties/>
    </property>
    <property name="room_object_id" type="class" propertytype="alveus_content::RoomObjectId">
     <properties>
      <property name=":variant" propertytype="alveus_content::RoomObjectId:::Variant" value="DietFridge"/>
     </properties>
    </property>
    <property name="open_menu" type="class" propertytype="alveus_interaction::OpenMenu">
     <properties>
    <property name="menu_id" type="class" propertytype="alveus_types::CareMenuId">
     <properties>
      <property name=":variant" propertytype="alveus_types::CareMenuId:::Variant" value="Fridge"/>
     </properties>
    </property>
      <property name="prompt" type="string" value="Open fridge"/>
     </properties>
    </property>   </properties>
 </tile>
 <tile id="6">
  <image source="tiles/seed_chest.png" width="32" height="32"/>
   <properties>
    <property name="obstacle" type="class" propertytype="alveus_components::Obstacle">
     <properties/>
    </property>
    <property name="room_object_id" type="class" propertytype="alveus_content::RoomObjectId">
     <properties>
      <property name=":variant" propertytype="alveus_content::RoomObjectId:::Variant" value="SeedChest"/>
     </properties>
    </property>
    <property name="give_item" type="class" propertytype="alveus_interaction::GiveItem">
     <properties>
    <property name="item_id" type="class" propertytype="alveus_types::ItemId">
     <properties>
      <property name=":variant" propertytype="alveus_types::ItemId:::Variant" value="ChickenGrains"/>
     </properties>
    </property>
      <property name="prompt" type="string" value="Scoop chicken grains"/>
     </properties>
    </property>   </properties>
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
    <property name="obstacle" type="class" propertytype="alveus_components::Obstacle">
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
    <property name="obstacle" type="class" propertytype="alveus_components::Obstacle">
     <properties/>
    </property>
   </properties>
 </tile>
 <tile id="12">
  <image source="tiles/feeding_dish.png" width="32" height="32"/>
   <properties>
    <property name="obstacle" type="class" propertytype="alveus_components::Obstacle">
     <properties/>
    </property>
    <property name="room_object_id" type="class" propertytype="alveus_content::RoomObjectId">
     <properties>
      <property name=":variant" propertytype="alveus_content::RoomObjectId:::Variant" value="PushPopFeedingDish"/>
     </properties>
    </property>
    <property name="feed_animal" type="class" propertytype="alveus_interaction::FeedAnimal">
     <properties>
    <property name="animal_id" type="class" propertytype="alveus_types::AnimalId">
     <properties>
      <property name=":variant" propertytype="alveus_types::AnimalId:::Variant" value="PushPop"/>
     </properties>
    </property>
    <property name="required_item" type="class" propertytype="alveus_types::ItemId">
     <properties>
      <property name=":variant" propertytype="alveus_types::ItemId:::Variant" value="TortoiseLeafyGreens"/>
     </properties>
    </property>
      <property name="delta" type="class" propertytype="alveus_types::FeedStat">
       <properties>
        <property name="0" type="class" propertytype="alveus_types::Stat">
         <properties>
          <property name="0" type="int" value="1000"/>
         </properties>
        </property>
       </properties>
      </property>
      <property name="prompt" type="string" value="Place leafy greens for Push Pop"/>
     </properties>
    </property>   </properties>
 </tile>
 <tile id="13">
  <image source="tiles/prep_table_chore.png" width="32" height="32"/>
   <properties>
    <property name="obstacle" type="class" propertytype="alveus_components::Obstacle">
     <properties/>
    </property>
    <property name="room_object_id" type="class" propertytype="alveus_content::RoomObjectId">
     <properties>
      <property name=":variant" propertytype="alveus_content::RoomObjectId:::Variant" value="PrepTable"/>
     </properties>
    </property>
    <property name="mini_chore" type="class" propertytype="alveus_interaction::MiniChore">
     <properties>
    <property name="chore_id" type="class" propertytype="alveus_types::ChoreId">
     <properties>
      <property name=":variant" propertytype="alveus_types::ChoreId:::Variant" value="ChopVeggies"/>
     </properties>
    </property>
    <property name="required_item" type="class" propertytype="core::option::Option&lt;alveus_types::ItemId&gt;">
     <properties>
      <property name=":variant" propertytype="core::option::Option&lt;alveus_types::ItemId&gt;:::Variant" value="Some"/>
      <property name="Some" type="class" propertytype="core::option::Option&lt;alveus_types::ItemId&gt;::Some">
       <properties>
    <property name="0" type="class" propertytype="alveus_types::ItemId">
     <properties>
      <property name=":variant" propertytype="alveus_types::ItemId:::Variant" value="RawVeggieTub"/>
     </properties>
    </property>
       </properties>
      </property>
     </properties>
    </property>
    <property name="output_item" type="class" propertytype="core::option::Option&lt;alveus_types::ItemId&gt;">
     <properties>
      <property name=":variant" propertytype="core::option::Option&lt;alveus_types::ItemId&gt;:::Variant" value="Some"/>
      <property name="Some" type="class" propertytype="core::option::Option&lt;alveus_types::ItemId&gt;::Some">
       <properties>
    <property name="0" type="class" propertytype="alveus_types::ItemId">
     <properties>
      <property name=":variant" propertytype="alveus_types::ItemId:::Variant" value="PreparedVeggieDiet"/>
     </properties>
    </property>
       </properties>
      </property>
     </properties>
    </property>
      <property name="prompt" type="string" value="Chop veggies"/>
     </properties>
    </property>   </properties>
 </tile>
 <tile id="14">
  <image source="tiles/toy_bin.png" width="32" height="32"/>
   <properties>
    <property name="obstacle" type="class" propertytype="alveus_components::Obstacle">
     <properties/>
    </property>
    <property name="room_object_id" type="class" propertytype="alveus_content::RoomObjectId">
     <properties>
      <property name=":variant" propertytype="alveus_content::RoomObjectId:::Variant" value="ToyBin"/>
     </properties>
    </property>
    <property name="give_item" type="class" propertytype="alveus_interaction::GiveItem">
     <properties>
    <property name="item_id" type="class" propertytype="alveus_types::ItemId">
     <properties>
      <property name=":variant" propertytype="alveus_types::ItemId:::Variant" value="MiniMirror"/>
     </properties>
    </property>
      <property name="prompt" type="string" value="Take mini mirror"/>
     </properties>
    </property>   </properties>
 </tile>
 <tile id="15">
  <image source="tiles/polly_feed_bowl.png" width="32" height="32"/>
   <properties>
    <property name="obstacle" type="class" propertytype="alveus_components::Obstacle">
     <properties/>
    </property>
    <property name="room_object_id" type="class" propertytype="alveus_content::RoomObjectId">
     <properties>
      <property name=":variant" propertytype="alveus_content::RoomObjectId:::Variant" value="PollyFeedBowl"/>
     </properties>
    </property>
    <property name="feed_animal" type="class" propertytype="alveus_interaction::FeedAnimal">
     <properties>
    <property name="animal_id" type="class" propertytype="alveus_types::AnimalId">
     <properties>
      <property name=":variant" propertytype="alveus_types::AnimalId:::Variant" value="Polly"/>
     </properties>
    </property>
    <property name="required_item" type="class" propertytype="alveus_types::ItemId">
     <properties>
      <property name=":variant" propertytype="alveus_types::ItemId:::Variant" value="ChickenGrains"/>
     </properties>
    </property>
      <property name="delta" type="class" propertytype="alveus_types::FeedStat">
       <properties>
        <property name="0" type="class" propertytype="alveus_types::Stat">
         <properties>
          <property name="0" type="int" value="1000"/>
         </properties>
        </property>
       </properties>
      </property>
      <property name="prompt" type="string" value="Fill Polly's bowl"/>
     </properties>
    </property>   </properties>
 </tile>
 <tile id="16">
  <image source="tiles/polly_nesting.png" width="32" height="32"/>
   <properties>
    <property name="obstacle" type="class" propertytype="alveus_components::Obstacle">
     <properties/>
    </property>
    <property name="room_object_id" type="class" propertytype="alveus_content::RoomObjectId">
     <properties>
      <property name=":variant" propertytype="alveus_content::RoomObjectId:::Variant" value="PollyNestingBox"/>
     </properties>
    </property>
    <property name="clean_animal" type="class" propertytype="alveus_interaction::CleanAnimal">
     <properties>
    <property name="animal_id" type="class" propertytype="alveus_types::AnimalId">
     <properties>
      <property name=":variant" propertytype="alveus_types::AnimalId:::Variant" value="Polly"/>
     </properties>
    </property>
    <property name="required_item" type="class" propertytype="core::option::Option&lt;alveus_types::ItemId&gt;">
     <properties>
      <property name=":variant" propertytype="core::option::Option&lt;alveus_types::ItemId&gt;:::Variant" value="None"/>
     </properties>
    </property>
      <property name="delta" type="class" propertytype="alveus_types::CleanStat">
       <properties>
        <property name="0" type="class" propertytype="alveus_types::Stat">
         <properties>
          <property name="0" type="int" value="1000"/>
         </properties>
        </property>
       </properties>
      </property>
      <property name="prompt" type="string" value="Sweep nesting"/>
     </properties>
    </property>   </properties>
 </tile>
 <tile id="17">
  <image source="tiles/polly_enrichment.png" width="32" height="32"/>
   <properties>
    <property name="obstacle" type="class" propertytype="alveus_components::Obstacle">
     <properties/>
    </property>
    <property name="room_object_id" type="class" propertytype="alveus_content::RoomObjectId">
     <properties>
      <property name=":variant" propertytype="alveus_content::RoomObjectId:::Variant" value="PollyEnrichmentPost"/>
     </properties>
    </property>
    <property name="enrich_animal" type="class" propertytype="alveus_interaction::EnrichAnimal">
     <properties>
    <property name="animal_id" type="class" propertytype="alveus_types::AnimalId">
     <properties>
      <property name=":variant" propertytype="alveus_types::AnimalId:::Variant" value="Polly"/>
     </properties>
    </property>
    <property name="required_item" type="class" propertytype="core::option::Option&lt;alveus_types::ItemId&gt;">
     <properties>
      <property name=":variant" propertytype="core::option::Option&lt;alveus_types::ItemId&gt;:::Variant" value="Some"/>
      <property name="Some" type="class" propertytype="core::option::Option&lt;alveus_types::ItemId&gt;::Some">
       <properties>
    <property name="0" type="class" propertytype="alveus_types::ItemId">
     <properties>
      <property name=":variant" propertytype="alveus_types::ItemId:::Variant" value="MiniMirror"/>
     </properties>
    </property>
       </properties>
      </property>
     </properties>
    </property>
      <property name="delta" type="class" propertytype="alveus_types::EnrichStat">
       <properties>
        <property name="0" type="class" propertytype="alveus_types::Stat">
         <properties>
          <property name="0" type="int" value="1000"/>
         </properties>
        </property>
       </properties>
      </property>
      <property name="prompt" type="string" value="Place mirror"/>
     </properties>
    </property>   </properties>
 </tile>
</tileset>
