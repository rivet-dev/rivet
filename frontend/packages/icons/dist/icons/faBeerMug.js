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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faBeerMug.js
var require_faBeerMug = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faBeerMug.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "beer-mug";
    var width = 512;
    var height = 512;
    var aliases = ["beer-foam"];
    var unicode = "e0b3";
    var svgPathData = "M267.7 133.3C253 149.7 231.7 160 208 160s-45-10.3-59.7-26.7c-6.9-7.7-21.4-8.6-29.6-2.3C108 139.1 94.6 144 80 144c-35.3 0-64-28.7-64-64s28.7-64 64-64c14.6 0 28 4.9 38.7 13c8.2 6.3 22.7 5.4 29.6-2.3C163 10.3 184.3 0 208 0s45 10.3 59.7 26.7c6.9 7.7 21.4 8.6 29.6 2.3C308 20.9 321.4 16 336 16c35.3 0 64 28.7 64 64s-28.7 64-64 64c-14.6 0-28-4.9-38.7-13c-8.2-6.3-22.7-5.4-29.6 2.3zM32 448l0-284.8C46.1 171.3 62.5 176 80 176c18.7 0 36.2-5.4 51-14.7c20 19 47.2 30.7 77 30.7s57-11.7 77-30.7c14.8 9.3 32.3 14.7 51 14.7c19.6 0 37.9-5.9 53.1-16l46.1 0c42.4 0 76.8 34.4 76.8 76.8l0 102.1c0 30.3-17.9 57.9-45.6 70.2L384 445.7l0 2.3c0 35.3-28.7 64-64 64L96 512c-35.3 0-64-28.7-64-64zm352-72.4l56.4-25.1c4.6-2.1 7.6-6.6 7.6-11.7l0-102.1c0-7.1-5.7-12.8-12.8-12.8L384 224l0 151.6zM160 240c0-8.8-7.2-16-16-16s-16 7.2-16 16l0 160c0 8.8 7.2 16 16 16s16-7.2 16-16l0-160zm64 0c0-8.8-7.2-16-16-16s-16 7.2-16 16l0 160c0 8.8 7.2 16 16 16s16-7.2 16-16l0-160zm64 0c0-8.8-7.2-16-16-16s-16 7.2-16 16l0 160c0 8.8 7.2 16 16 16s16-7.2 16-16l0-160z";
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
    exports.faBeerMug = exports.definition;
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

// src/_entries/faBeerMug.js
var import_faBeerMug = __toESM(require_faBeerMug());
var export_faBeerMug = import_faBeerMug.definition;
export {
  export_faBeerMug as faBeerMug
};
