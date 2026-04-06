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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faSteeringWheel.js
var require_faSteeringWheel = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faSteeringWheel.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "steering-wheel";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f622";
    var svgPathData = "M224 358.9l0 86.5C143.6 431.9 80.1 368.4 66.7 288l102.2 0L224 358.9zm64 0L343.1 288l102.2 0C431.9 368.4 368.4 431.9 288 445.3l0-86.5zM352 224l-7.2-14.3c-5.4-10.8-16.5-17.7-28.6-17.7l-120.4 0c-12.1 0-23.2 6.8-28.6 17.7L160 224l-93.3 0C81.9 133.2 160.9 64 256 64s174.1 69.2 189.3 160L352 224zM256 512A256 256 0 1 0 256 0a256 256 0 1 0 0 512z";
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
    exports.faSteeringWheel = exports.definition;
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

// src/_entries/faSteeringWheel.js
var import_faSteeringWheel = __toESM(require_faSteeringWheel());
var export_faSteeringWheel = import_faSteeringWheel.definition;
export {
  export_faSteeringWheel as faSteeringWheel
};
