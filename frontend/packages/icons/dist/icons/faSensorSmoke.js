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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faSensorCloud.js
var require_faSensorCloud = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faSensorCloud.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "sensor-cloud";
    var width = 640;
    var height = 512;
    var aliases = ["sensor-smoke"];
    var unicode = "e02c";
    var svgPathData = "M64 32C28.7 32 0 60.7 0 96L0 416c0 35.3 28.7 64 64 64l170.8 0c-6.9-14.5-10.8-30.8-10.8-48c0-44.7 26.2-83.3 64.1-101.3C290.9 271.3 339.9 224 400 224c17.2 0 33.5 3.9 48 10.8L448 96c0-35.3-28.7-64-64-64L64 32zm48 88l0 144c0 13.3-10.7 24-24 24s-24-10.7-24-24l0-144c0-13.3 10.7-24 24-24s24 10.7 24 24zm96 0l0 144c0 13.3-10.7 24-24 24s-24-10.7-24-24l0-144c0-13.3 10.7-24 24-24s24 10.7 24 24zM336 512l224 0c44.2 0 80-35.8 80-80c0-39-27.9-71.5-64.8-78.6c.5-3.1 .8-6.2 .8-9.4c0-30.9-25.1-56-56-56c-18.1 0-34.1 8.6-44.4 21.8C464.8 278.5 435 256 400 256c-44.2 0-80 35.8-80 80c0 5.9 .6 11.7 1.9 17.2C284.4 359.9 256 392.6 256 432c0 44.2 35.8 80 80 80z";
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
    exports.faSensorCloud = exports.definition;
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

// src/_entries/faSensorSmoke.js
var import_faSensorCloud = __toESM(require_faSensorCloud());
var export_faSensorSmoke = import_faSensorCloud.definition;
export {
  export_faSensorSmoke as faSensorSmoke
};
