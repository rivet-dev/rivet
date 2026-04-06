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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faHeat.js
var require_faHeat = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faHeat.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "heat";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "e00c";
    var svgPathData = "M176 32c17.7 0 32 14.3 32 32l0 80c0 34.6 11.2 68.3 32 96l19.2 25.6C288.3 304.4 304 351.5 304 400l0 48c0 17.7-14.3 32-32 32s-32-14.3-32-32l0-48c0-34.6-11.2-68.3-32-96l-19.2-25.6C159.7 239.6 144 192.5 144 144l0-80c0-17.7 14.3-32 32-32zM32 96c17.7 0 32 14.3 32 32l0 32c0 34.6 11.2 68.3 32 96l19.2 25.6C144.3 320.4 160 367.5 160 416l0 32c0 17.7-14.3 32-32 32s-32-14.3-32-32l0-32c0-34.6-11.2-68.3-32-96L44.8 294.4C15.7 255.6 0 208.5 0 160l0-32c0-17.7 14.3-32 32-32zm320 32l0 32c0 34.6 11.2 68.3 32 96l19.2 25.6C432.3 320.4 448 367.5 448 416l0 32c0 17.7-14.3 32-32 32s-32-14.3-32-32l0-32c0-34.6-11.2-68.3-32-96l-19.2-25.6C303.7 255.6 288 208.5 288 160l0-32c0-17.7 14.3-32 32-32s32 14.3 32 32z";
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
    exports.faHeat = exports.definition;
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

// src/_entries/faHeat.js
var import_faHeat = __toESM(require_faHeat());
var export_faHeat = import_faHeat.definition;
export {
  export_faHeat as faHeat
};
