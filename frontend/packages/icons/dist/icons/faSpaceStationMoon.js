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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faSpaceStationMoon.js
var require_faSpaceStationMoon = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faSpaceStationMoon.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "space-station-moon";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e033";
    var svgPathData = "M512 256c0 7.8-.3 15.5-1 23.1C438.8 304.7 351.2 320 256 320s-182.8-15.3-255-40.9c-.7-7.6-1-15.3-1-23.1C0 114.6 114.6 0 256 0S512 114.6 512 256zM256 512C134.9 512 33.4 427.9 6.8 314.8C79.4 338.4 164.7 352 256 352s176.6-13.6 249.2-37.2C478.6 427.9 377.1 512 256 512zM160 160a32 32 0 1 1 64 0 32 32 0 1 1 -64 0zm112 0a80 80 0 1 0 -160 0 80 80 0 1 0 160 0z";
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
    exports.faSpaceStationMoon = exports.definition;
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

// src/_entries/faSpaceStationMoon.js
var import_faSpaceStationMoon = __toESM(require_faSpaceStationMoon());
var export_faSpaceStationMoon = import_faSpaceStationMoon.definition;
export {
  export_faSpaceStationMoon as faSpaceStationMoon
};
