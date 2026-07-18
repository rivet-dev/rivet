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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faBoxingGlove.js
var require_faBoxingGlove = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faBoxingGlove.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "boxing-glove";
    var width = 448;
    var height = 512;
    var aliases = [129354, "glove-boxing"];
    var unicode = "f438";
    var svgPathData = "M0 96C0 43 43 0 96 0L288 0c53 0 96 43 96 96l0 32-32 0c-26.2 0-49.4 12.6-64 32l-144 0c-21.2 0-34.3-5.3-41.8-9.7c-3.8-2.3-6.3-4.4-7.7-5.8c-.7-.7-1.1-1.2-1.3-1.5l-.1-.1s0 0 0 0c-5-7.1-14.7-9-22-4.2c-7.4 4.9-9.3 14.8-4.4 22.2L80 152c-13.3 8.9-13.3 8.9-13.3 8.9s0 0 0 0s0 0 0 0l.1 .1 .1 .2 .4 .5c.3 .4 .6 .8 1.1 1.4c.9 1.1 2 2.4 3.6 4c3.1 3.1 7.6 6.9 13.8 10.7C98.3 185.3 117.2 192 144 192l129.6 0c-1 5.2-1.6 10.5-1.6 16c0 37.3 25.5 68.6 60 77.5c8.6 2.2 17.3-2.9 19.5-11.5s-2.9-17.3-11.5-19.5c-20.7-5.3-36-24.1-36-46.5c0-26.5 21.5-48 48-48l32 0c35.3 0 64 28.7 64 64l0 24.2c0 25.5-10.1 49.9-28.1 67.9l-49.1 49.1c-12 12-18.7 28.3-18.7 45.3l0 37.5c0 35.3-28.7 64-64 64L96 512c-35.3 0-64-28.7-64-64l0-88.4C12.4 342 0 316.4 0 288L0 96zM152.9 354.7c-7.4-4.9-17.3-2.9-22.2 4.4s-2.9 17.3 4.4 22.2l28 18.7-28 18.7c-7.4 4.9-9.3 14.8-4.4 22.2s14.8 9.3 22.2 4.4L192 419.2l39.1 26.1c7.4 4.9 17.3 2.9 22.2-4.4s2.9-17.3-4.4-22.2l-28-18.7 28-18.7c7.4-4.9 9.3-14.8 4.4-22.2s-14.8-9.3-22.2-4.4L192 380.8l-39.1-26.1z";
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
    exports.faBoxingGlove = exports.definition;
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

// src/_entries/faGloveBoxing.js
var import_faBoxingGlove = __toESM(require_faBoxingGlove());
var export_faGloveBoxing = import_faBoxingGlove.definition;
export {
  export_faGloveBoxing as faGloveBoxing
};
