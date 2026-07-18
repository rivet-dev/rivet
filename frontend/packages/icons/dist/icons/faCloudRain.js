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

// src/node_modules/@fortawesome/free-solid-svg-icons/faCloudRain.js
var require_faCloudRain = __commonJS({
  "src/node_modules/@fortawesome/free-solid-svg-icons/faCloudRain.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "cloud-rain";
    var width = 512;
    var height = 512;
    var aliases = [127783, 9926];
    var unicode = "f73d";
    var svgPathData = "M96 320c-53 0-96-43-96-96 0-42.5 27.6-78.6 65.9-91.2-1.3-6.7-1.9-13.7-1.9-20.8 0-61.9 50.1-112 112-112 43.1 0 80.5 24.3 99.2 60 14.7-17.1 36.5-28 60.8-28 44.2 0 80 35.8 80 80 0 5.5-.6 10.8-1.6 16 .5 0 1.1 0 1.6 0 53 0 96 43 96 96s-43 96-96 96L96 320zm1.6 68.2c1.1-2.5 3.6-4.2 6.4-4.2s5.3 1.6 6.4 4.2l30.2 68.2c2.2 5.1 3.4 10.5 3.4 16 0 21.9-18.1 39.6-40 39.6s-40-17.7-40-39.6c0-5.5 1.2-11 3.4-16l30.2-68.2zm152 0c1.1-2.5 3.6-4.2 6.4-4.2s5.3 1.6 6.4 4.2l30.2 68.2c2.2 5.1 3.4 10.5 3.4 16 0 21.9-18.1 39.6-40 39.6s-40-17.7-40-39.6c0-5.5 1.2-11 3.4-16l30.2-68.2zm121.8 68.2l30.2-68.2c1.1-2.5 3.6-4.2 6.4-4.2s5.3 1.6 6.4 4.2l30.2 68.2c2.2 5.1 3.4 10.5 3.4 16 0 21.9-18.1 39.6-40 39.6s-40-17.7-40-39.6c0-5.5 1.2-11 3.4-16z";
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
    exports.faCloudRain = exports.definition;
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

// src/_entries/faCloudRain.js
var import_faCloudRain = __toESM(require_faCloudRain());
var export_faCloudRain = import_faCloudRain.definition;
export {
  export_faCloudRain as faCloudRain
};
