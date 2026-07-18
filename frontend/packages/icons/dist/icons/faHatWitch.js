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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faHatWitch.js
var require_faHatWitch = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faHatWitch.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "hat-witch";
    var width = 576;
    var height = 512;
    var aliases = [];
    var unicode = "f6e7";
    var svgPathData = "M200.7 181.4L112 384l112 0 0-48c0-26.5 21.5-48 48-48l32 0c26.5 0 48 21.5 48 48l0 48 112 0L386.7 206.1c-1.8-4-2.7-8.4-2.7-12.8l0-.8c0-5.6 1.5-11 4.2-15.9l18.6-32.5c5.7-10 16.3-16.1 27.8-16.1l20.4 0c14.7 0 27.5 10 31 24.2l6.2 24.6c2.2 8.9 10.2 15.1 19.4 15.1c9.5 0 17.7-6.7 19.6-16.1l12.2-60.8c.4-2.1 .6-4.2 .6-6.3l0-6.8c0-9-3.8-17.5-10.4-23.6L457.2 8.4C451.3 3 443.6 0 435.6 0l-2.2 0c-6.1 0-12.1 1.8-17.3 5L273.1 96.6c-32 20.5-57.2 50-72.4 84.8zM320 384l0-48c0-8.8-7.2-16-16-16l-32 0c-8.8 0-16 7.2-16 16l0 48 64 0zM22.6 416C10.1 416 0 426.1 0 438.6c0 6 2.4 11.7 6.9 15.6C25.9 470.3 82.6 512 160 512l256 0c77.4 0 134.1-41.7 153.1-57.8c4.6-3.9 6.9-9.6 6.9-15.6c0-12.5-10.1-22.6-22.6-22.6L22.6 416z";
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
    exports.faHatWitch = exports.definition;
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

// src/_entries/faHatWitch.js
var import_faHatWitch = __toESM(require_faHatWitch());
var export_faHatWitch = import_faHatWitch.definition;
export {
  export_faHatWitch as faHatWitch
};
