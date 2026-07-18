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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faDolphin.js
var require_faDolphin = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faDolphin.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "dolphin";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e168";
    var svgPathData = "M477.3 24.9c3.5-5.3 3.6-12.2 .1-17.6S467.7-.8 461.4 .2c-16.7 2.8-43.1 8.6-68.8 17C378 22 362.5 28 349 35.3C306.5 12.8 258 0 206.5 0L176 0C96.5 0 32 64.5 32 144c0 18.6 3.5 36.4 10 52.7L20.5 209.6C7.8 217.2 0 231 0 245.8C0 269.1 18.9 288 42.2 288L176 288l16 0 11.2 0 92 61.3c4.9 3.3 11.2 3.6 16.4 .8s8.5-8.2 8.5-14.1l0-48 6.6 0c31.7 0 57.4 25.7 57.4 57.4c0 30.6-24 55.8-54.5 57.3l-53.4 2.7c-20.2-33-56.3-53.4-95.4-53.4l-4.7 0c-5.5 0-10.7 2.9-13.6 7.6s-3.2 10.6-.7 15.6L190.1 432l-28.4 56.8c-2.5 5-2.2 10.8 .7 15.6s8.1 7.6 13.6 7.6l4.7 0c35.9 0 69.2-17.2 90.1-45.4l68 3.8c94.1 5.2 173.3-70 173.3-164.3c0-80.8-31.4-154.5-82.7-209.3l48-72zM176 120a24 24 0 1 1 0 48 24 24 0 1 1 0-48z";
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
    exports.faDolphin = exports.definition;
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

// src/_entries/faDolphin.js
var import_faDolphin = __toESM(require_faDolphin());
var export_faDolphin = import_faDolphin.definition;
export {
  export_faDolphin as faDolphin
};
