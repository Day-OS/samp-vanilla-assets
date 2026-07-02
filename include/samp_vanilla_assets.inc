#if defined _samp_vanilla_assets_included
	#endinput
#endif
#define _samp_vanilla_assets_included

#include <open.mp>
#include <streamer>

enum E_SCREEN_MODEL {
	SCREEN_MODEL_STANDARD = 0,
	SCREEN_MODEL_SHADOW = 1
};

#define INVALID_SCREEN_INDEX (-1)

// Create3DMediaScreen works out image/gif/video/youtube-live on its own from
// `url` - playerid only matters for video/live (it's who the extracted audio
// targets); tileCols/tileRows size the mosaic and apply to any media kind.
// modelId chooses the 3D model (see E_SCREEN_MODEL enum).
// Returns a screenIndex handle - keep it if you want to Destroy3DMediaScreen
// it later, it stays valid for the screen's whole lifetime.
native Create3DMediaScreen(const url[], Float:x, Float:y, Float:z, Float:rotationX = 0.0, Float:rotationY = 0.0, Float:rotationZ = 0.0, tileCols = 1, tileRows = 1, playerid = -1, world_id = -1, interior_id = -1, Float:audioRange = 5.0, Float:hiddenX = 0.0, Float:hiddenY = 0.0, Float:hiddenZ = 0.0, E_SCREEN_MODEL:modelId = SCREEN_MODEL_STANDARD);
// Removes every object backing the given screenIndex. Returns 0 if the index
// is out of range or was already destroyed.
native Destroy3DMediaScreen(screenIndex);
native SVA_AreaListenerOnPlayerEnter(playerid, areaid);
native SVA_AreaListenerOnPlayerLeave(playerid, areaid);

native Create3DMediaScreenPreview(Float:x, Float:y, Float:z, Float:rotationX = 0.0, Float:rotationY = 0.0, Float:rotationZ = 0.0, tileCols = 1, tileRows = 1, worldid = -1, interior_id = -1, playerid = -1);
native Destroy3DMediaScreenPreview(previewObjectId);

native CreateDialogScreen(playerid, const url[], cols = 32, rows = 32);
native DestroyDialogScreen(screenIndex);

// CreateTextDrawScreen renders media as a HUD overlay via PlayerTextDraws.
native CreateTextDrawScreen(const url[], playerid, Float:x, Float:y, cols = 64, rows = 64, Float:letterSizeX = 0.25, Float:letterSizeY = 0.35, Float:boxScale = 9.0, budget = 256);
native DestroyTextDrawScreen(screenIndex);

// Screens use dynamic areas to know when a player enters/leaves their trigger
// radius (audio range etc.), but those areas only ever notify Pawn through
// the ordinary OnPlayerEnterDynamicArea/OnPlayerLeaveDynamicArea callbacks -
// so every user of this library would otherwise have to remember to forward
// both callbacks into SVA_AreaListenerOnPlayerEnter/Leave by hand. Instead we
// define them here and rename the identifier below (the same ALS technique
// streamer.inc/foreach.inc use) so a script that also wants its own
// OnPlayerEnterDynamicArea/OnPlayerLeaveDynamicArea keeps working untouched -
// its `public` gets silently renamed to the *_hook symbol and chained in.
forward OnPlayerEnterDynamicArea(playerid, STREAMER_TAG_AREA:areaid);
public OnPlayerEnterDynamicArea(playerid, STREAMER_TAG_AREA:areaid)
{
	SVA_AreaListenerOnPlayerEnter(playerid, _:areaid);
	#if defined SVA_OnPlayerEnterDynamicArea_hook
		SVA_OnPlayerEnterDynamicArea_hook(playerid, areaid);
	#endif
	return 1;
}
#if defined _ALS_OnPlayerEnterDynamicArea
	#undef OnPlayerEnterDynamicArea
#else
	#define _ALS_OnPlayerEnterDynamicArea
#endif
#define OnPlayerEnterDynamicArea SVA_OnPlayerEnterDynamicArea_hook

forward OnPlayerLeaveDynamicArea(playerid, STREAMER_TAG_AREA:areaid);
public OnPlayerLeaveDynamicArea(playerid, STREAMER_TAG_AREA:areaid)
{
	SVA_AreaListenerOnPlayerLeave(playerid, _:areaid);
	#if defined SVA_OnPlayerLeaveDynamicArea_hook
		SVA_OnPlayerLeaveDynamicArea_hook(playerid, areaid);
	#endif
	return 1;
}
#if defined _ALS_OnPlayerLeaveDynamicArea
	#undef OnPlayerLeaveDynamicArea
#else
	#define _ALS_OnPlayerLeaveDynamicArea
#endif
#define OnPlayerLeaveDynamicArea SVA_OnPlayerLeaveDynamicArea_hook

// The plugin backs every screen type with objects, materials, textdraws and
// dialogs it creates and calls into by name at runtime (via amx_FindNative on
// this script's own AMX) - not by calling these natives from Pawn code. The
// AMX compiler strips a native's entry from the compiled file's name table if
// nothing in the script reaches it, so without this dead call the plugin
// would fail to resolve them the moment a script only used, say,
// CreateTextDrawScreen and nothing else that happens to reference
// CreateObject or ShowPlayerDialog. Keeping the reference here means anyone
// who includes this file gets every dependency kept alive for free.
forward _SVA_KeepDependencyNativesAlive();
public _SVA_KeepDependencyNativesAlive()
{
	#pragma warning push
	#pragma warning disable 205
	if (false)
	{
		CreateObject(0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
		new objectid = CreatePlayerObject(0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
		SetObjectMaterialText(objectid, "", 0, OBJECT_MATERIAL_SIZE_256x256, "Arial", 48, true, 0, 0, OBJECT_MATERIAL_TEXT_ALIGN_CENTRE);
		SetDynamicObjectPos(objectid, 0.0, 0.0, 0.0);
		SetObjectMaterial(objectid, 0, 0, "", "", 0xFFFFFFFF);
		new modelid, texlib[16], texname[16], colour;
		GetObjectMaterial(objectid, 0, modelid, texlib, sizeof(texlib), texname, sizeof(texname), colour);
		AddSimpleModel(-1, 0, -1, "none.dff", "none.txd");
		PlayAudioStreamForPlayer(0, "", 0.0, 0.0, 0.0, 50.0, false);
		AttachDynamicObjectToObject(objectid, objectid, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
		DestroyObject(objectid);
		new players[MAX_PLAYERS];
		GetPlayers(players, sizeof(players));
		new Float:x, Float:y, Float:z;
		GetPlayerPos(0, x, y, z);
		StopAudioStreamForPlayer(0);
		new Text:logo = TextDrawCreate(0.0, 0.0, "");
		TextDrawFont(logo, TEXT_DRAW_FONT_0);
		TextDrawTextSize(logo, 0.0, 0.0);
		TextDrawShowForAll(logo);
		TextDrawShowForPlayer(0, logo);
		ShowPlayerDialog(0, 0, DIALOG_STYLE_MSGBOX, "", "", "", "");
		HidePlayerDialog(0);
		new PlayerText:ptd = CreatePlayerTextDraw(0, 0.0, 0.0, ".");
		PlayerTextDrawDestroy(0, ptd);
		PlayerTextDrawLetterSize(0, ptd, 0.0, 0.0);
		PlayerTextDrawTextSize(0, ptd, 0.0, 0.0);
		PlayerTextDrawAlignment(0, ptd, 1);
		PlayerTextDrawColour(0, ptd, 0xFFFFFFFF);
		PlayerTextDrawBackgroundColour(0, ptd, 0);
		PlayerTextDrawSetOutline(0, ptd, 0);
		PlayerTextDrawSetShadow(0, ptd, 0);
		PlayerTextDrawFont(0, ptd, 0);
		PlayerTextDrawShow(0, ptd);
		PlayerTextDrawHide(0, ptd);
		PlayerTextDrawSetString(0, ptd, ".");
		PlayerTextDrawBoxColor(0, 0, 0);
		PlayerTextDrawUseBox(0, 0, false);
		CreateDynamicObject(0, 0, 0, 0, 0, 0, 0, -1, -1, -1, 200.0, 0.0, -1, 0);
		SetDynamicObjectMaterialText(objectid, 0, "", OBJECT_MATERIAL_SIZE_256x128, "Arial", 24,  1, 0xFFFFFFFF, 0, 0);
		new STREAMER_TAG_AREA:areaid = CreateDynamicSphere(0.0, 0.0, 0.0, 5.0, -1, -1, -1);
		new STREAMER_TAG_AREA:circleid = CreateDynamicCircle(0.0, 0.0, 5.0, -1, -1, -1, 0);
		new STREAMER_TAG_AREA:cylinderid = CreateDynamicCylinder(0.0, 0.0, -5.0, 5.0, 5.0, -1, -1, -1);
		IsPlayerInDynamicArea(0, areaid, true);
		DestroyDynamicArea(circleid);
		DestroyDynamicArea(cylinderid);
		DestroyDynamicArea(areaid);
		EditPlayerObject(0, objectid);
	}
	#pragma warning pop
	return 1;
}
