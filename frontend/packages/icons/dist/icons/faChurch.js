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

// src/node_modules/@fortawesome/free-solid-svg-icons/faChurch.js
var require_faChurch = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faChurch.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "church";
    var width = 512;
    var height = 512;
    var aliases = [9962];
    var unicode = "f51d";
    var svgPathData = "M280-8c0-13.3-10.7-24-24-24S232-21.3 232-8l0 24-32 0c-13.3 0-24 10.7-24 24s10.7 24 24 24l32 0 0 48-98.6 65.8C120 186.7 112 201.6 112 217.7l0 38.3-79.8 45.6C12.3 313 0 334.2 0 357.1L0 448c0 35.3 28.7 64 64 64 74.7 0 85.4 0 320 0l64 0c35.3 0 64-28.7 64-64l0-90.9c0-23-12.3-44.2-32.2-55.6L400 256 400 217.7c0-16-8-31-21.4-39.9l-98.6-65.8 0-48 32 0c13.3 0 24-10.7 24-24s-10.7-24-24-24l-32 0 0-24zM256 320c35.3 0 64 28.7 64 64l0 80-128 0 0-80c0-35.3 28.7-64 64-64z";
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
    exports.faChurch = exports.definition;
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

// src/_entries/faChurch.js
var import_faChurch = __toESM(require_faChurch());
var export_faChurch = import_faChurch.definition;
export {
  export_faChurch as faChurch
};
