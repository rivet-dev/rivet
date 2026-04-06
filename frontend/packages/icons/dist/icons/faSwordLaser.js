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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faSwordLaser.js
var require_faSwordLaser = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faSwordLaser.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "sword-laser";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e03b";
    var svgPathData = "M505 7c9.1 9.1 9.4 23.7 .7 33.2L285.2 281.3l-54.4-54.4L471.8 6.3c9.5-8.7 24.1-8.3 33.2 .7zM195.3 236.7l80 80c6.2 6.2 6.2 16.4 0 22.6c-6.1 6.1-15.8 6.2-22.1 .5l-81-81c-5.8-6.3-5.6-16 .5-22.1c6.2-6.2 16.4-6.2 22.6 0zm-57.7 57.7l23.7-23.7 80 80-23.7 23.7-80-80zm-11.3 11.3l80 80-20.7 20.7-80-80 20.7-20.7zM87 330.3L181.7 425l-2.3 2.3L105 501.7C98.3 508.3 89.4 512 80 512s-18.3-3.7-25-10.3L10.3 457C3.7 450.3 0 441.4 0 432s3.7-18.3 10.3-25l74.3-74.3 2.3-2.3z";
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
    exports.faSwordLaser = exports.definition;
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

// src/_entries/faSwordLaser.js
var import_faSwordLaser = __toESM(require_faSwordLaser());
var export_faSwordLaser = import_faSwordLaser.definition;
export {
  export_faSwordLaser as faSwordLaser
};
