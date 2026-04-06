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

// src/node_modules/@fortawesome/free-solid-svg-icons/faLandMineOn.js
var require_faLandMineOn = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faLandMineOn.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "land-mine-on";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "e51b";
    var svgPathData = "M312 0l0 128c0 13.3-10.7 24-24 24s-24-10.7-24-24L264 0c0-13.3 10.7-24 24-24s24 10.7 24 24zM160 288c0-17.7 14.3-32 32-32l192 0c17.7 0 32 14.3 32 32l0 32 80 0c26.5 0 48 21.5 48 48l0 96c0 26.5-21.5 48-48 48L80 512c-26.5 0-48-21.5-48-48l0-96c0-26.5 21.5-48 48-48l80 0 0-32zM12 114.7c7.4-11 22.3-14 33.3-6.7l96 64c11 7.4 14 22.3 6.7 33.3s-22.3 14-33.3 6.7l-96-64c-11-7.4-14-22.3-6.7-33.3zM530.7 108c11-7.4 25.9-4.4 33.3 6.7s4.4 25.9-6.7 33.3l-96 64c-11 7.4-25.9 4.4-33.3-6.7s-4.4-25.9 6.7-33.3l96-64z";
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
    exports.faLandMineOn = exports.definition;
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

// src/_entries/faLandMineOn.js
var import_faLandMineOn = __toESM(require_faLandMineOn());
var export_faLandMineOn = import_faLandMineOn.definition;
export {
  export_faLandMineOn as faLandMineOn
};
