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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faFishCooked.js
var require_faFishCooked = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faFishCooked.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "fish-cooked";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "f7fe";
    var svgPathData = "M320 64c120 0 256 128 256 192s-136 192-256 192c-74.8 0-155.9-49.8-206.3-102.8c-6-6.3-15.8-7.2-22.5-1.7L26.1 396.4c-5.3 4.3-12.9 4.8-18.7 1.1s-8.6-10.7-6.9-17.4L30.5 259.9c.6-2.5 .6-5.2 0-7.8L.5 131.9c-1.6-6.5 .9-13.2 6.4-17s12.7-3.8 18.2 0l72 49.5c6.6 4.5 15.5 3.5 21.1-2.2C168.9 110.9 247.4 64 320 64zm-4.7 84.7c-6.2-6.2-16.4-6.2-22.6 0l-64 64c-6.2 6.2-6.2 16.4 0 22.6s16.4 6.2 22.6 0l64-64c6.2-6.2 6.2-16.4 0-22.6zm112 16c-6.2-6.2-16.4-6.2-22.6 0l-160 160c-6.2 6.2-6.2 16.4 0 22.6s16.4 6.2 22.6 0l160-160c6.2-6.2 6.2-16.4 0-22.6zm16 112c-6.2-6.2-16.4-6.2-22.6 0l-64 64c-6.2 6.2-6.2 16.4 0 22.6s16.4 6.2 22.6 0l64-64c6.2-6.2 6.2-16.4 0-22.6z";
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
    exports.faFishCooked = exports.definition;
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

// src/_entries/faFishCooked.js
var import_faFishCooked = __toESM(require_faFishCooked());
var export_faFishCooked = import_faFishCooked.definition;
export {
  export_faFishCooked as faFishCooked
};
