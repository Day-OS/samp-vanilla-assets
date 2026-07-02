/*
    Example filterscript for the samp_vanilla_assets plugin.

    Shows the full flow: pass a media URL with a command, drag/rotate a
    preview ("ghost") with the mouse via the native object editor, then
    create the real screen wherever you let go.

    Commands: /screen <media url>, /imgdialog <media url>, /tdscreen <media url>, /delscreen
*/

#define FILTERSCRIPT

#include <open.mp>
#include <streamer>
#include <samp_vanilla_assets>

#define SCREEN_TILE_COLS 4
#define SCREEN_TILE_ROWS 4
#define SCREEN_HIDDEN_BUFFER_Z_OFFSET (8.0)
#define DIALOG_SCREEN_DIALOG_ID 1
#define DIALOG_SCREEN_COLS 48
#define DIALOG_SCREEN_ROWS 48

// Default AMX dynamic memory is tiny (a holdover from ancient hardware
// constraints); the Rust plugin allocates large strings on this same heap for
// the pixel-grid sign, so give it room to spare instead of tuning around a
// 2005-era default every time the grid gets bigger.
#pragma dynamic 1048576

new
    bool:gPendingScreenPlacement[MAX_PLAYERS],
    gPendingScreenUrl[MAX_PLAYERS][128],
    gPendingScreenGhost[MAX_PLAYERS],
    gLastScreenIndex[MAX_PLAYERS],
    gLastDialogScreenIndex[MAX_PLAYERS],
    gLastTdScreenIndex[MAX_PLAYERS],
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
    gLastTdScreenIndex[playerid] = INVALID_SCREEN_INDEX;
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

// Just a demonstration that OnPlayerEnterDynamicArea/OnPlayerLeaveDynamicArea
// keep working as normal callbacks after #include <samp_vanilla_assets> - the
// library already forwarded these into its own area-listener natives before
// this runs, no extra wiring needed here.
public OnPlayerEnterDynamicArea(playerid, STREAMER_TAG_AREA:areaid)
{
    printf("[samp_led_demo] player %d entered dynamic area %d", playerid, _:areaid);
    return 1;
}

public OnPlayerLeaveDynamicArea(playerid, STREAMER_TAG_AREA:areaid)
{
    printf("[samp_led_demo] player %d left dynamic area %d", playerid, _:areaid);
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

    if (strcmp(cmdtext, "/tdscreen ", true, 10) == 0)
    {
        strmid(url, cmdtext, 10, strlen(cmdtext));
        if (gLastTdScreenIndex[playerid] != INVALID_SCREEN_INDEX)
        {
            DestroyTextDrawScreen(gLastTdScreenIndex[playerid]);
        }
        gLastTdScreenIndex[playerid] = CreateTextDrawScreen(url, playerid, 120.0, 120.0, .cols = 64, .rows = 64, .letterSizeX = 0.30, .letterSizeY = 0.30);
        SendClientMessage(playerid, 0xFFFFFFFF, "TextDraw screen criada.");
        return 1;
    }
    if (strcmp(cmdtext, "/tdscreen", true) == 0)
    {
        SendClientMessage(playerid, 0xFFFFFFFF, "Usage: /tdscreen <media url>");
        return 1;
    }
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
