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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faSpaceStationMoonConstruction.js
var require_faSpaceStationMoonConstruction = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faSpaceStationMoonConstruction.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "space-station-moon-construction";
    var width = 512;
    var height = 512;
    var aliases = ["space-station-moon-alt"];
    var unicode = "e034";
    var svgPathData = "M512 256c0 7.8-.3 15.5-1 23.1C438.8 304.7 351.2 320 256 320s-182.8-15.3-255-40.9c-.7-7.6-1-15.3-1-23.1C0 114.6 114.6 0 256 0c57.8 0 111.1 19.1 153.9 51.4c3.9 2.9 6.1 7.5 6.1 12.4L416 80c0 8.8-7.2 16-16 16l-32 0c-8.8 0-16 7.2-16 16l0 32c0 8.8 7.2 16 16 16l48 0 16 0c8.8 0 16 7.2 16 16l0 32c0 8.8 7.2 16 16 16l31.9 0c8.1 0 14.9 6 15.5 14.1c.4 5.9 .6 11.9 .6 17.9zM256 512C134.9 512 33.4 427.9 6.8 314.8C79.4 338.4 164.7 352 256 352c44.3 0 87.3-3.2 128-9.2l0 25.2c0 8.8 7.2 16 16 16l32 0c8.8 0 16 7.2 16 16l0 19.3c0 3.9-1.4 7.7-4.1 10.5C397.2 480.4 330.3 512 256 512zM160 160a32 32 0 1 1 64 0 32 32 0 1 1 -64 0zm112 0a80 80 0 1 0 -160 0 80 80 0 1 0 160 0z";
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
    exports.faSpaceStationMoonConstruction = exports.definition;
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

// src/_entries/faSpaceStationMoonConstruction.js
var import_faSpaceStationMoonConstruction = __toESM(require_faSpaceStationMoonConstruction());
var export_faSpaceStationMoonConstruction = import_faSpaceStationMoonConstruction.definition;
export {
  export_faSpaceStationMoonConstruction as faSpaceStationMoonConstruction
};
