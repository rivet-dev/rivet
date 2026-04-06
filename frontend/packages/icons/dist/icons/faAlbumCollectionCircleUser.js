var __create = Object.create;
var __defProp = Object.defineProperty;
var __getOwnPropDesc = Object.getOwnPropertyDescriptor;
var __getOwnPropNames = Object.getOwnPropertyNames;
var __getProtoOf = Object.getPrototypeOf;
var __hasOwnProp = Object.prototype.hasOwnProperty;
var __commonJS = (cb, mod) => function __require() {
  return mod || (0, cb[__getOwnPropNames(cb)[0]])((mod = { exports: {} }).exports, mod), mod.exports;
};
var __copyProps = (to, from, except, desc) => {
  if (from && typeof from === "object" || typeof from === "function") {
    for (let key of __getOwnPropNames(from))
      if (!__hasOwnProp.call(to, key) && key !== except)
        __defProp(to, key, { get: () => from[key], enumerable: !(desc = __getOwnPropDesc(from, key)) || desc.enumerable });
  }
  return to;
};
var __toESM = (mod, isNodeMode, target) => (target = mod != null ? __create(__getProtoOf(mod)) : {}, __copyProps(
  // If the importer is in node compatibility mode or this is not an ESM
  // file that has been converted to a CommonJS file using a Babel-
  // compatible transform (i.e. "__esModule" has not been set), then set
  // "default" to the CommonJS "module.exports" for node compatibility.
  isNodeMode || !mod || !mod.__esModule ? __defProp(target, "default", { value: mod, enumerable: true }) : target,
  mod
));

// src/node_modules/@fortawesome/pro-solid-svg-icons/faAlbumCollectionCircleUser.js
var require_faAlbumCollectionCircleUser = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faAlbumCollectionCircleUser.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "album-collection-circle-user";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e48f";
    var svgPathData = "M32 24c0 13.3 10.7 24 24 24l400 0c13.3 0 24-10.7 24-24s-10.7-24-24-24L56 0C42.7 0 32 10.7 32 24zM48 160c-13.8 0-26.9 5.9-36 16.2S-1.3 200.3 .4 214l32 256c3 24 23.4 42 47.6 42l314.8 0c-18.9-13.3-35.1-30.3-47.5-49.9c-26.9 11.4-58 17.9-91.3 17.9C154.4 480 72 419.1 72 344s82.4-136 184-136c45.4 0 86.9 12.1 119 32.2c31.5-29.9 74.2-48.2 121-48.2c4.5 0 9 .2 13.4 .5c-2-5.9-5.2-11.5-9.4-16.3c-9.1-10.3-22.2-16.2-36-16.2L48 160zM256 376c17.7 0 32-10.7 32-24s-14.3-24-32-24s-32 10.7-32 24s14.3 24 32 24zM40 80c-13.3 0-24 10.7-24 24s10.7 24 24 24l432 0c13.3 0 24-10.7 24-24s-10.7-24-24-24L40 80zM640 368a144 144 0 1 0 -288 0 144 144 0 1 0 288 0zm-66.3 80.7C553.5 468.1 526.2 480 496 480s-57.5-11.9-77.7-31.3c6.2-19 24-32.7 45.1-32.7l65.2 0c21 0 38.9 13.7 45.1 32.7zM448 336a48 48 0 1 1 96 0 48 48 0 1 1 -96 0z";
    exports.definition = {
      prefix,
      iconName,
      icon: [
        width,
        height,
        aliases,
        unicode,
        svgPathData
      ]
    };
    exports.faAlbumCollectionCircleUser = exports.definition;
    exports.prefix = prefix;
    exports.iconName = iconName;
    exports.width = width;
    exports.height = height;
    exports.ligatures = aliases;
    exports.unicode = unicode;
    exports.svgPathData = svgPathData;
    exports.aliases = aliases;
  }
});

// src/_entries/faAlbumCollectionCircleUser.js
var import_faAlbumCollectionCircleUser = __toESM(require_faAlbumCollectionCircleUser());
var export_faAlbumCollectionCircleUser = import_faAlbumCollectionCircleUser.definition;
export {
  export_faAlbumCollectionCircleUser as faAlbumCollectionCircleUser
};
