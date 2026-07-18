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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faBusSchool.js
var require_faBusSchool = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faBusSchool.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "bus-school";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "f5dd";
    var svgPathData = "M256 0C380.8 0 464 35.2 464 80l0 16 0 16 16 0c17.7 0 32 14.3 32 32l0 64c0 17.7-14.3 32-32 32l-16 0 0 28.2c9.8 8.8 16 21.6 16 35.8l0 96c0 20.9-13.4 38.7-32 45.3l0 34.7c0 17.7-14.3 32-32 32l-32 0c-17.7 0-32-14.3-32-32l0-32-192 0 0 32c0 17.7-14.3 32-32 32l-32 0c-17.7 0-32-14.3-32-32l0-34.7c-18.6-6.6-32-24.4-32-45.3l0-96c0-14.2 6.2-27 16-35.8L48 240l-16 0c-17.7 0-32-14.3-32-32l0-64c0-17.7 14.3-32 32-32l16 0 0-16 0-16C48 35.2 131.2 0 256 0zM112 160l0 64c0 17.7 14.3 32 32 32l96 0 0-128-96 0c-17.7 0-32 14.3-32 32zm160 96l96 0c17.7 0 32-14.3 32-32l0-64c0-17.7-14.3-32-32-32l-96 0 0 128zM112 384a32 32 0 1 0 0-64 32 32 0 1 0 0 64zm288 0a32 32 0 1 0 0-64 32 32 0 1 0 0 64zM352 80c0-8.8-7.2-16-16-16L176 64c-8.8 0-16 7.2-16 16s7.2 16 16 16l160 0c8.8 0 16-7.2 16-16z";
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
    exports.faBusSchool = exports.definition;
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

// src/_entries/faBusSchool.js
var import_faBusSchool = __toESM(require_faBusSchool());
var export_faBusSchool = import_faBusSchool.definition;
export {
  export_faBusSchool as faBusSchool
};
