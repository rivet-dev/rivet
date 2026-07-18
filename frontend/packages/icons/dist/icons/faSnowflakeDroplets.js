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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faSnowflakeDroplets.js
var require_faSnowflakeDroplets = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faSnowflakeDroplets.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "snowflake-droplets";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e5c1";
    var svgPathData = "M320 0c17.7 0 32 14.3 32 32l0 30.1 15-15c9.4-9.4 24.6-9.4 33.9 0s9.4 24.6 0 33.9l-49 49 0 70.3 61.4-35.8 17.7-66.1c3.4-12.8 16.6-20.4 29.4-17s20.4 16.6 17 29.4l-5.2 19.3 23.6-13.8c15.3-8.9 34.9-3.7 43.8 11.5s3.7 34.9-11.5 43.8l-25.3 14.8 21.7 5.8c12.8 3.4 20.4 16.6 17 29.4s-16.6 20.4-29.4 17l-67.7-18.1L383.5 256l60.9 35.5 67.7-18.1c12.8-3.4 26 4.2 29.4 17s-4.2 26-17 29.4l-21.7 5.8 21.5 12.6-29.3 57-22.7-13.2 5.2 19.3c3.4 12.8-4.2 26-17 29.4s-26-4.2-29.4-17l-17.7-66.1L352 311.7l0 70.3 49 49c9.4 9.4 9.4 24.6 0 33.9s-24.6 9.4-33.9 0l-15-15 0 30.1c0 17.7-14.3 32-32 32s-32-14.3-32-32l0-30.1-15 15c-9.4 9.4-24.6 9.4-33.9 0s-9.4-24.6 0-33.9l49-49 0-70.3-61.4 35.8-17.7 66.1c-3.4 12.8-16.6 20.4-29.4 17s-20.4-16.6-17-29.4l5.2-19.3L145 395.1l-29.3-57 21.5-12.6-21.7-5.8c-12.8-3.4-20.4-16.6-17-29.4s16.6-20.4 29.4-17l67.7 18.1L256.5 256l-60.9-35.5-67.7 18.1c-12.8 3.4-26-4.2-29.4-17s4.2-26 17-29.4l21.7-5.8-25.3-14.8c-15.3-8.9-20.4-28.5-11.5-43.8s28.5-20.4 43.8-11.5l23.6 13.8-5.2-19.3c-3.4-12.8 4.2-26 17-29.4s26 4.2 29.4 17l17.7 66.1L288 200.3l0-70.3L239 81c-9.4-9.4-9.4-24.6 0-33.9s24.6-9.4 33.9 0l15 15L288 32c0-17.7 14.3-32 32-32zM519.1 418.1l47.5-92.4c1.8-3.5 5.4-5.7 9.3-5.7s7.5 2.2 9.3 5.7l47.5 92.4c4.7 9.1 7.1 19.3 7.1 29.5c0 35.3-28.7 64.3-64 64.3s-64-29-64-64.3c0-10.2 2.4-20.4 7.1-29.5zM54.7 325.7c1.8-3.5 5.4-5.7 9.3-5.7s7.5 2.2 9.3 5.7l47.5 92.4c4.7 9.1 7.1 19.3 7.1 29.5C128 483 99.3 512 64 512s-64-29-64-64.3c0-10.2 2.4-20.4 7.1-29.5l47.5-92.4z";
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
    exports.faSnowflakeDroplets = exports.definition;
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

// src/_entries/faSnowflakeDroplets.js
var import_faSnowflakeDroplets = __toESM(require_faSnowflakeDroplets());
var export_faSnowflakeDroplets = import_faSnowflakeDroplets.definition;
export {
  export_faSnowflakeDroplets as faSnowflakeDroplets
};
