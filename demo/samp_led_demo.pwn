/*
    Example filterscript for the samp-led plugin.

    Shows the full flow: pass a media URL with a command, drag/rotate a
    preview ("ghost") with the mouse via the native object editor, then
    create the real screen wherever you let go.

    Commands: /screen <media url>, /imgdialog <media url>, /delscreen
*/

#define FILTERSCRIPT

#include <open.mp>
#include <streamer>

// Create3DMediaScreen works out image/gif/video/youtube-live on its own from
// `url` - playerid only matters for video/live (it's who the extracted audio
// targets); tileCols/tileRows size the mosaic and apply to any media kind.
// Returns a screenIndex handle - keep it if you want to Destroy3DMediaScreen
// it later, it stays valid for the screen's whole lifetime.
native Create3DMediaScreen(const url[], Float:x, Float:y, Float:z, Float:rotationX = 0.0, Float:rotationY = 0.0, Float:rotationZ = 0.0, tileCols = 1, tileRows = 1, playerid = -1, world_id = -1, interior_id = -1, Float:audioRange = 5.0, Float:hiddenX = 0.0, Float:hiddenY = 0.0, Float:hiddenZ = 0.0);
// Removes every object backing the given screenIndex. Returns 0 if the index
// is out of range or was already destroyed.
native Destroy3DMediaScreen(screenIndex);
native SVA_AreaListenerOnPlayerEnter(playerid, areaid);
native SVA_AreaListenerOnPlayerLeave(playerid, areaid);

native Create3DMediaScreenPreview(Float:x, Float:y, Float:z, Float:rotationX = 0.0, Float:rotationY = 0.0, Float:rotationZ = 0.0, tileCols = 1, tileRows = 1, worldid = -1, interior_id = -1, playerid = -1);
native Destroy3DMediaScreenPreview(previewObjectId);
native CreateDialogScreen(playerid, const url[], cols = 32, rows = 32);
native DestroyDialogScreen(screenIndex);

#define INVALID_SCREEN_INDEX (-1)

#define SCREEN_TILE_COLS 1
#define SCREEN_TILE_ROWS 1
#define SCREEN_HIDDEN_BUFFER_Z_OFFSET (8.0)
#define DIALOG_SCREEN_DIALOG_ID 1
#define DIALOG_SCREEN_COLS 48
#define DIALOG_SCREEN_ROWS 48

// Default AMX dynamic memory is tiny (a holdover from ancient hardware
// constraints); the Rust plugin allocates large strings on this same heap for
// the pixel-grid sign, so give it room to spare instead of tuning around a
// 2005-era default every time the grid gets bigger.
#pragma dynamic 1048576

//The AMX only registers natives that are
// referenced somewhere in the compiled script. This unreachable reference
// keeps all seven in this script's import table so the plugin can resolve
// them at runtime.
forward _KeepObjectNativesAlive();
public _KeepObjectNativesAlive()
{
    #pragma warning push
    #pragma warning disable 205
    if (false)
    {
        new objectid = CreateObject(0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
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
        CreateDynamicObject(0, 0, 0, 0, 0, 0, 0, -1, -1, -1, 200.0, 0.0, -1, 0);
        SetDynamicObjectMaterialText(objectid, 0, "", OBJECT_MATERIAL_SIZE_256x128, "Arial", 24,  1, 0xFFFFFFFF, 0, 0);
        new STREAMER_TAG_AREA:areaid = CreateDynamicSphere(0.0, 0.0, 0.0, 5.0, -1, -1, -1);
        new STREAMER_TAG_AREA:circleid = CreateDynamicCircle(0.0, 0.0, 5.0, -1, -1, -1, 0);
        new STREAMER_TAG_AREA:cylinderid = CreateDynamicCylinder(0.0, 0.0, -5.0, 5.0, 5.0, -1, -1, -1);
        IsPlayerInDynamicArea(0, areaid, true);
        DestroyDynamicArea(circleid);
        DestroyDynamicArea(cylinderid);
        DestroyDynamicArea(areaid);
    }
    #pragma warning pop
    return 1;
}

new
    bool:gPendingScreenPlacement[MAX_PLAYERS],
    gPendingScreenUrl[MAX_PLAYERS][128],
    gPendingScreenGhost[MAX_PLAYERS],
    gLastScreenIndex[MAX_PLAYERS],
    gLastDialogScreenIndex[MAX_PLAYERS],
    gDynamicProbeObject = INVALID_STREAMER_ID
;

public OnFilterScriptInit()
{
    // Runtime probe: if this succeeds, CreateDynamicObject is loaded and callable.
    gDynamicProbeObject = CreateDynamicObject(19353, 0.0, 0.0, 10.0, 0.0, 0.0, 0.0, -1, -1, -1, 200.0, 0.0, -1, 0);
    if (gDynamicProbeObject != INVALID_STREAMER_ID)
    {
        print("CreateDynamicObject OK (runtime probe created).");
    }
    else
    {
        print("CreateDynamicObject FAILED (runtime probe not created).");
    }

    print("samp_led_demo loaded - commands: /screen <media url> /imgdialog <media url> /delscreen");
    return 1;
}

public OnFilterScriptExit()
{
    if (gDynamicProbeObject != INVALID_STREAMER_ID)
    {
        DestroyDynamicObject(gDynamicProbeObject);
        gDynamicProbeObject = INVALID_STREAMER_ID;
    }

    return 1;
}

public OnPlayerConnect(playerid)
{
    gPendingScreenPlacement[playerid] = false;
    gPendingScreenGhost[playerid] = INVALID_OBJECT_ID;
    gLastScreenIndex[playerid] = INVALID_SCREEN_INDEX;
    gLastDialogScreenIndex[playerid] = INVALID_SCREEN_INDEX;
    return 1;
}

public OnPlayerDisconnect(playerid, reason)
{
    if (gPendingScreenPlacement[playerid])
    {
        Destroy3DMediaScreenPreview(gPendingScreenGhost[playerid]);
        gPendingScreenPlacement[playerid] = false;
    }
    if (gLastDialogScreenIndex[playerid] != INVALID_SCREEN_INDEX)
    {
        DestroyDialogScreen(gLastDialogScreenIndex[playerid]);
        gLastDialogScreenIndex[playerid] = INVALID_SCREEN_INDEX;
    }
    return 1;
}

public OnPlayerEnterDynamicArea(playerid, STREAMER_TAG_AREA:areaid)
{
    SVA_AreaListenerOnPlayerEnter(playerid, _:areaid);
    return 1;
}

public OnPlayerLeaveDynamicArea(playerid, STREAMER_TAG_AREA:areaid)
{
    SVA_AreaListenerOnPlayerLeave(playerid, _:areaid);
    return 1;
}

// Spawns a draggable/rotatable preview of the screen in front of the player
// and remembers what to create once they confirm the placement in
// OnPlayerEditObject. The ghost's starting spot is just a convenience - the
// player can move/rotate it anywhere before confirming.
StartScreenPlacement(playerid, const url[])
{
    if (strlen(url) == 0)
    {
        SendClientMessage(playerid, 0xFFFFFFFF, "Usage: /screen <media url>");
        return;
    }

    new Float:x, Float:y, Float:z, Float:rotationZ;
    GetPlayerPos(playerid, x, y, z);
    GetPlayerFacingAngle(playerid, rotationZ);
    x += 2.0 * floatsin(rotationZ, degrees);
    y += 2.0 * floatcos(rotationZ, degrees);

    gPendingScreenPlacement[playerid] = true;
    strcopy(gPendingScreenUrl[playerid], url);
    gPendingScreenGhost[playerid] = Create3DMediaScreenPreview(x, y, z, 0.0, 0.0, rotationZ, SCREEN_TILE_COLS, SCREEN_TILE_ROWS, -1, -1, playerid);
    EditDynamicObject(playerid, gPendingScreenGhost[playerid]);
    SendClientMessage(playerid, 0xFFFFFFFF, "Arraste/gire a tela e clique em concluir para confirmar.");
}

public OnPlayerCommandText(playerid, cmdtext[])
{
    new url[128];

    if (strcmp(cmdtext, "/imgdialog ", true, 11) == 0)
    {
        strmid(url, cmdtext, 11, strlen(cmdtext));
        if (gLastDialogScreenIndex[playerid] != INVALID_SCREEN_INDEX)
        {
            DestroyDialogScreen(gLastDialogScreenIndex[playerid]);
        }

        gLastDialogScreenIndex[playerid] = CreateDialogScreen(playerid, url, DIALOG_SCREEN_COLS, DIALOG_SCREEN_ROWS);
        SendClientMessage(playerid, 0xFFFFFFFF, "Dialog screen criada.");
        return 1;
    }
    if (strcmp(cmdtext, "/imgdialog", true) == 0)
    {
        SendClientMessage(playerid, 0xFFFFFFFF, "Usage: /imgdialog <media url>");
        return 1;
    }
    if (strcmp(cmdtext, "/screen ", true, 8) == 0)
    {
        strmid(url, cmdtext, 8, strlen(cmdtext));
        StartScreenPlacement(playerid, url);
        return 1;
    }
    if (strcmp(cmdtext, "/screen", true) == 0)
    {
        SendClientMessage(playerid, 0xFFFFFFFF, "Usage: /screen <media url>");
        return 1;
    }
    if (strcmp(cmdtext, "/delscreen", true) == 0)
    {
        if (gLastScreenIndex[playerid] == INVALID_SCREEN_INDEX)
        {
            SendClientMessage(playerid, 0xFFFFFFFF, "Nenhuma tela pra apagar.");
            return 1;
        }

        if (Destroy3DMediaScreen(gLastScreenIndex[playerid]))
        {
            SendClientMessage(playerid, 0xFFFFFFFF, "Tela apagada.");
        }
        else
        {
            SendClientMessage(playerid, 0xFFFFFFFF, "Essa tela ja nao existe mais.");
        }

        gLastScreenIndex[playerid] = INVALID_SCREEN_INDEX;
        return 1;
    }
    return 0;
}

public OnDialogResponse(playerid, dialogid, response, listitem, inputtext[])
{
    if (dialogid == DIALOG_SCREEN_DIALOG_ID && response == 1)
    {
        if (gLastDialogScreenIndex[playerid] != INVALID_SCREEN_INDEX)
        {
            DestroyDialogScreen(gLastDialogScreenIndex[playerid]);
            gLastDialogScreenIndex[playerid] = INVALID_SCREEN_INDEX;
        }
        return 1;
    }

    return 0;
}

public OnPlayerEditDynamicObject(playerid, STREAMER_TAG_OBJECT:objectid, response, Float:x, Float:y, Float:z, Float:rx, Float:ry, Float:rz)
{
    if (objectid != gPendingScreenGhost[playerid] || !gPendingScreenPlacement[playerid])
        return 0;

    Destroy3DMediaScreenPreview(_:objectid);
    gPendingScreenGhost[playerid] = INVALID_OBJECT_ID;

    if (response == _:EDIT_RESPONSE_FINAL)
    {
        new url[128];
        strcopy(url, gPendingScreenUrl[playerid]);

        gLastScreenIndex[playerid] = Create3DMediaScreen(url, x, y, z, rx, ry, rz, SCREEN_TILE_COLS, SCREEN_TILE_ROWS, -1, -1, -1, 5.0, x, y, z + SCREEN_HIDDEN_BUFFER_Z_OFFSET);
        SendClientMessage(playerid, 0xFFFFFFFF, "Tela criada! Use /delscreen para apaga-la.");
    }
    else
    {
        SendClientMessage(playerid, 0xFFFFFFFF, "Posicionamento cancelado.");
    }

    gPendingScreenPlacement[playerid] = false;
    return 1;
}
