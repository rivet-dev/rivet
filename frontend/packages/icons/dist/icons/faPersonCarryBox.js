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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faPersonCarryBox.js
var require_faPersonCarryBox = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faPersonCarryBox.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "person-carry-box";
    var width = 384;
    var height = 512;
    var aliases = ["person-carry"];
    var unicode = "f4cf";
    var svgPathData = "M80 96A48 48 0 1 0 80 0a48 48 0 1 0 0 96zM96 306.7l0-65.1 16.6 24.9c8.9 13.4 23.9 21.4 39.9 21.4l71.4 0 128 0c17.7 0 32-14.3 32-32l0-128c0-17.7-14.3-32-32-32L224 96c-17.7 0-32 14.3-32 32l0 96-30.9 0-42.6-64c-13.3-20-35.8-32-59.9-32C26.2 128 0 154.2 0 186.6l0 88.1c0 17 6.7 33.3 18.7 45.3l79.4 79.4 14.3 85.9c2.9 17.4 19.4 29.2 36.8 26.3s29.2-19.4 26.3-36.8l-15.2-90.9c-1.6-9.9-6.3-19-13.4-26.1l-51-51zM0 346.5L0 480c0 17.7 14.3 32 32 32s32-14.3 32-32c0 0 0 0 0 0l0-69.5-64-64z";
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
    exports.faPersonCarryBox = exports.definition;
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

// src/_entries/faPersonCarryBox.js
var import_faPersonCarryBox = __toESM(require_faPersonCarryBox());
var export_faPersonCarryBox = import_faPersonCarryBox.definition;
export {
  export_faPersonCarryBox as faPersonCarryBox
};
