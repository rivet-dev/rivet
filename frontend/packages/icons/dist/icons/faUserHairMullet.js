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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faUserHairMullet.js
var require_faUserHairMullet = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faUserHairMullet.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "user-hair-mullet";
    var width = 448;
    var height = 512;
    var aliases = ["business-front", "party-back", "trian-balbot"];
    var unicode = "e45c";
    var svgPathData = "M304 128l0 16c0 18-6 34.6-16 48c0 0 0 0 0 0s0 0 0 0c-14.6 19.4-37.8 32-64 32s-49.4-12.6-64-32c0 0 0 0 0 0s0 0 0 0c-10-13.4-16-30-16-48l0-16c0-5.7 .6-11.3 1.7-16.6c30.6-3.1 58.5-17.9 78.3-40.6c19.8 22.7 47.6 37.5 78.3 40.6c1.1 5.4 1.7 10.9 1.7 16.6zM237.4 271.3L255 289c5.5 5.5 13.2 8 20.8 6.7s14.2-6.1 17.6-13l6.1-12.2 19.5 7.8c9.3 3.7 19.8 1.3 26.6-6s8.3-18.1 3.8-27c-8.5-17-11-22.3-12.2-27.1c-.7-2.9-1.1-5.9-1.2-12.3C346.2 187.5 352 166.4 352 144l0-16c0-7.1-.6-14-1.7-20.7c5.9-4.4 9.7-11.4 9.7-19.3c0-13.3-10.7-24-24-24l-1.1 0C312.7 25.7 271.4 0 224 0s-88.7 25.7-110.9 64L112 64C98.7 64 88 74.7 88 88c0 7.9 3.8 14.9 9.7 19.3C96.6 114 96 120.9 96 128l0 16c0 22.4 5.8 43.5 15.9 61.8c-.1 6.4-.5 9.4-1.2 12.3c-1.2 4.8-3.7 10.1-12.2 27.1c-4.5 8.9-2.9 19.7 3.8 27s17.3 9.7 26.6 6l19.5-7.8 6.1 12.2c3.5 6.9 10 11.7 17.6 13s15.4-1.3 20.8-6.7l17.7-17.7c4.4 .5 8.9 .7 13.4 .7s9-.2 13.4-.7zM161.3 320C72.2 320 0 392.2 0 481.3c0 17 13.8 30.7 30.7 30.7l386.6 0c17 0 30.7-13.8 30.7-30.7C448 392.2 375.8 320 286.7 320l-125.4 0z";
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
    exports.faUserHairMullet = exports.definition;
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

// src/_entries/faUserHairMullet.js
var import_faUserHairMullet = __toESM(require_faUserHairMullet());
var export_faUserHairMullet = import_faUserHairMullet.definition;
export {
  export_faUserHairMullet as faUserHairMullet
};
