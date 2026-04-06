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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faRollerCoaster.js
var require_faRollerCoaster = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faRollerCoaster.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "roller-coaster";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e324";
    var svgPathData = "M64 190.1c0-31.8 15.8-60 40-77L104 456c0 13.3 10.7 24 24 24s24-10.7 24-24l0-359.8c2-.1 4-.2 6.1-.2c14.8 0 29.1 3.5 41.9 9.9L200 456c0 13.3 10.7 24 24 24s24-10.7 24-24l0-301.3 48 69.1L296 456c0 13.3 10.7 24 24 24s24-10.7 24-24l0-163.1 20.3 29.2c7.9 11.3 17.2 21.3 27.7 29.8L392 456c0 13.3 10.7 24 24 24s24-10.7 24-24l0-78.4c13.6 4.2 28 6.4 42.6 6.4l5.4 0 0 72c0 13.3 10.7 24 24 24s24-10.7 24-24l0-77.6c14.4-4.2 27.8-10.5 40-18.6l0 88.2c0 17.7 14.3 32 32 32s32-14.3 32-32l0-200-.2 0c.1-2.6 .2-5.3 .2-8c0-79.5-64.5-144-144-144l-4 0c-28.9 0-57.3 7.1-82.8 20.7l-51.6 27.5L432 251.3l0-74.2 7.3-3.9c12.7-6.8 26.5-11 40.7-12.5L480 264c0 13.3 10.7 24 24 24s24-10.7 24-24l0-97.3c28.3 12.3 48 40.5 48 73.3c0 44.2-35.8 80-80 80l-13.4 0c-26.2 0-50.8-12.8-65.7-34.4L287.9 99.9C258.4 57.4 209.9 32 158.1 32C70.8 32 0 102.8 0 190.1L0 448c0 17.7 14.3 32 32 32s32-14.3 32-32l0-257.9z";
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
    exports.faRollerCoaster = exports.definition;
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

// src/_entries/faRollerCoaster.js
var import_faRollerCoaster = __toESM(require_faRollerCoaster());
var export_faRollerCoaster = import_faRollerCoaster.definition;
export {
  export_faRollerCoaster as faRollerCoaster
};
