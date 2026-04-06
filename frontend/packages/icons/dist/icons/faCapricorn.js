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

// src/node_modules/@fortawesome/free-solid-svg-icons/faCapricorn.js
var require_faCapricorn = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faCapricorn.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "capricorn";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "e848";
    var svgPathData = "M240 32c79.5 0 144 64.5 144 144l0 65.2c18.8-10.9 40.7-17.2 64-17.2 70.7 0 128 57.3 128 128S518.7 480 448 480c-35.8 0-68.2-14.7-91.4-38.4-28.8 42.5-77.4 70.4-132.6 70.4-17.7 0-32-14.3-32-32s14.3-32 32-32c53 0 96-43 96-96l0-176c0-44.2-35.8-80-80-80s-80 35.8-80 80l0 208c0 17.7-14.3 32-32 32s-32-14.3-32-32l0-224c0-35.3-28.7-64-64-64-17.7 0-32-14.3-32-32S14.3 32 32 32C72.6 32 108.8 51 132.2 80.5 158.6 50.7 197.1 32 240 32zM448 288a64 64 0 1 0 0 128 64 64 0 1 0 0-128z";
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
    exports.faCapricorn = exports.definition;
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

// src/_entries/faCapricorn.js
var import_faCapricorn = __toESM(require_faCapricorn());
var export_faCapricorn = import_faCapricorn.definition;
export {
  export_faCapricorn as faCapricorn
};
