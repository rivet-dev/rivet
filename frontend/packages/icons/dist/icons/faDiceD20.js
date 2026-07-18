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

// src/node_modules/@fortawesome/free-solid-svg-icons/faDiceD20.js
var require_faDiceD20 = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faDiceD20.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "dice-d20";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f6cf";
    var svgPathData = "M224.4-8.2c19.6-11.1 43.6-11.1 63.1 0l192 108.8c20 11.4 32.4 32.6 32.4 55.7l0 215.6c0 23-12.4 44.3-32.4 55.7l-192 108.8c-19.6 11.1-43.6 11.1-63.1 0L32.4 427.5C12.4 416.1 0 394.8 0 371.8L0 156.2c0-23 12.4-44.3 32.4-55.7L224.4-8.2zm52 73.2C267 49.8 245 49.8 235.6 65l-76.6 123.7-85.4-46.3-3.8-1.6c-8.9-2.7-18.8 1.1-23.4 9.6s-2.4 18.9 4.7 24.8l3.3 2.3 83.4 45.2-74.6 120.6C55.3 356.2 61 373 75 378.4l161 61.9 0 39.7c0 11 9 20 20 20s20-9 20-20l0-39.7 161-61.9c14-5.4 19.7-22.2 11.8-35l-74.7-120.6 83.4-45.2c9.7-5.3 13.3-17.4 8.1-27.1s-17.4-13.3-27.1-8.1L353 188.7 276.4 65zm-47 329.9l-122-46.9 54.5-88.1 67.5 135zM404.6 348l-122 46.9 67.5-135 54.5 88.1zM319.3 232L256 358.6 192.7 232 319.3 232zM308 192l-104.1 0 52-84 52 84z";
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
    exports.faDiceD20 = exports.definition;
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

// src/_entries/faDiceD20.js
var import_faDiceD20 = __toESM(require_faDiceD20());
var export_faDiceD20 = import_faDiceD20.definition;
export {
  export_faDiceD20 as faDiceD20
};
