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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faTrainTrack.js
var require_faTrainTrack = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faTrainTrack.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "train-track";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "e453";
    var svgPathData = "M222.4 74.1c5.6-16.8-3.5-34.9-20.2-40.5s-34.9 3.5-40.5 20.2L147.6 96 112 96c-17.7 0-32 14.3-32 32s14.3 32 32 32l14.3 0-21.3 64L72 224c-17.7 0-32 14.3-32 32s14.3 32 32 32l11.6 0L62.3 352 32 352c-17.7 0-32 14.3-32 32s14.3 32 32 32l8.9 0-7.3 21.9c-5.6 16.8 3.5 34.9 20.2 40.5s34.9-3.5 40.5-20.2l14-42.1 359.2 0 14 42.1c5.6 16.8 23.7 25.8 40.5 20.2s25.8-23.7 20.2-40.5L535.1 416l8.9 0c17.7 0 32-14.3 32-32s-14.3-32-32-32l-30.3 0-21.3-64 11.6 0c17.7 0 32-14.3 32-32s-14.3-32-32-32l-32.9 0-21.3-64 14.3 0c17.7 0 32-14.3 32-32s-14.3-32-32-32l-35.6 0-14-42.1c-5.6-16.8-23.7-25.8-40.5-20.2s-25.8 23.7-20.2 40.5L360.9 96 215.1 96l7.3-21.9zM193.7 160l188.5 0 21.3 64-231.2 0 21.3-64zM446.3 352l-316.5 0 21.3-64 273.9 0 21.3 64z";
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
    exports.faTrainTrack = exports.definition;
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

// src/_entries/faTrainTrack.js
var import_faTrainTrack = __toESM(require_faTrainTrack());
var export_faTrainTrack = import_faTrainTrack.definition;
export {
  export_faTrainTrack as faTrainTrack
};
