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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faSkeletonRibs.js
var require_faSkeletonRibs = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faSkeletonRibs.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "skeleton-ribs";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e5cb";
    var svgPathData = "M352 32c0-17.7-14.3-32-32-32s-32 14.3-32 32l0 32-27.8 0c-33.5 0-65.7 17.6-82.1 48.9C150.5 165.8 96 282.2 96 384l0 17c0 43.6 35.4 79 79 79c15.1 0 30.1-2.7 44.2-8l67.1-25.2c.6-.2 1.1-.4 1.7-.6l0 33.8c0 17.7 14.3 32 32 32s32-14.3 32-32l0-33.8c.6 .2 1.1 .4 1.7 .6L420.8 472c14.1 5.3 29.1 8 44.2 8c43.6 0 79-35.4 79-79l0-17c0-101.8-54.5-218.2-82.2-271.1C445.5 81.6 413.3 64 379.8 64L352 64l0-32zm-64 96l0 38.3-72.7 16.2c7-15.2 13.8-28.7 19.6-40c4.5-8.6 13.9-14.5 25.3-14.5l27.8 0zM186 254.6l102-22.7 0 39.7L171.2 303.4c4.1-16.5 9.2-32.9 14.7-48.8zM160.3 372.7L288 337.9l0 42c-8.2 1.7-16.3 4-24.2 7l-67.1 25.2c-7 2.6-14.3 3.9-21.7 3.9c-8.3 0-15-6.7-15-15l0-17c0-3.7 .1-7.5 .3-11.3zM352 379.9l0-40.9 118.8 35.6c3 .9 6.1 1.3 9.1 1.4c.1 2.7 .1 5.3 .1 8l0 17c0 8.3-6.7 15-15 15c-7.4 0-14.8-1.3-21.7-3.9l-67.1-25.2c-7.9-3-16-5.3-24.2-7zm117.8-72.4L352 272.2l0-40.3 102 22.7c6 17.2 11.4 35.1 15.7 53zm-45-125L352 166.3l0-38.3 27.8 0c11.4 0 20.8 5.9 25.3 14.5c5.9 11.3 12.6 24.8 19.6 40zM0 144l0 96L0 480c0 17.7 14.3 32 32 32s32-14.3 32-32l0-240c0-10.1 4.7-19.6 12.8-25.6l25.6-19.2C118.5 183.1 128 164.1 128 144c0-35.3-28.7-64-64-64S0 108.7 0 144zM576 80c-35.3 0-64 28.7-64 64c0 20.1 9.5 39.1 25.6 51.2l25.6 19.2c8.1 6 12.8 15.5 12.8 25.6l0 240c0 17.7 14.3 32 32 32s32-14.3 32-32l0-240 0-96c0-35.3-28.7-64-64-64z";
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
    exports.faSkeletonRibs = exports.definition;
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

// src/_entries/faSkeletonRibs.js
var import_faSkeletonRibs = __toESM(require_faSkeletonRibs());
var export_faSkeletonRibs = import_faSkeletonRibs.definition;
export {
  export_faSkeletonRibs as faSkeletonRibs
};
