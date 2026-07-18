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

// src/node_modules/@fortawesome/free-solid-svg-icons/faHouseMedicalFlag.js
var require_faHouseMedicalFlag = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faHouseMedicalFlag.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "house-medical-flag";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e514";
    var svgPathData = "M480 0c17.7 0 32 14.3 32 32l96 0c17.7 0 32 14.3 32 32l0 96c0 17.7-14.3 32-32 32l-96 0 0 288c0 17.7-14.3 32-32 32s-32-14.3-32-32l0-448c0-17.7 14.3-32 32-32zM267.5 59.1L400 186.9 400 503.4c-9.4 5.4-20.3 8.6-32 8.6l-256 0c-35.3 0-64-28.7-64-64l0-144-19.9 0C12.6 304 0 291.4 0 275.9 0 268.3 3.1 261 8.6 255.7L212.5 59.1C219.9 52 229.7 48 240 48s20.1 4 27.5 11.1zM204.8 256.8l0 44-44 0c-9.7 0-17.6 7.9-17.6 17.6l0 35.2c0 9.7 7.9 17.6 17.6 17.6l44 0 0 44c0 9.7 7.9 17.6 17.6 17.6l35.2 0c9.7 0 17.6-7.9 17.6-17.6l0-44 44 0c9.7 0 17.6-7.9 17.6-17.6l0-35.2c0-9.7-7.9-17.6-17.6-17.6l-44 0 0-44c0-9.7-7.9-17.6-17.6-17.6l-35.2 0c-9.7 0-17.6 7.9-17.6 17.6z";
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
    exports.faHouseMedicalFlag = exports.definition;
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

// src/_entries/faHouseMedicalFlag.js
var import_faHouseMedicalFlag = __toESM(require_faHouseMedicalFlag());
var export_faHouseMedicalFlag = import_faHouseMedicalFlag.definition;
export {
  export_faHouseMedicalFlag as faHouseMedicalFlag
};
