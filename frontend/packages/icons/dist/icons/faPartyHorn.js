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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faPartyHorn.js
var require_faPartyHorn = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faPartyHorn.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "party-horn";
    var width = 512;
    var height = 512;
    var aliases = [];
    var unicode = "e31b";
    var svgPathData = "M32 32a32 32 0 1 1 64 0A32 32 0 1 1 32 32zM448 160a32 32 0 1 1 64 0 32 32 0 1 1 -64 0zm32 256a32 32 0 1 1 0 64 32 32 0 1 1 0-64zM167 153c-9.4-9.4-9.4-24.6 0-33.9l8.3-8.3c16.7-16.7 27.2-38.6 29.8-62.1l3-27.4C209.6 8.2 221.5-1.3 234.7 .1s22.7 13.3 21.2 26.5l-3 27.4c-3.8 34.3-19.2 66.3-43.6 90.7L201 153c-9.4 9.4-24.6 9.4-33.9 0zM359 311l8.2-8.3c24.4-24.4 56.4-39.8 90.7-43.6l27.4-3c13.2-1.5 25 8 26.5 21.2s-8 25-21.2 26.5l-27.4 3c-23.5 2.6-45.4 13.1-62.1 29.8L393 345c-9.4 9.4-24.6 9.4-33.9 0s-9.4-24.6 0-33.9zM506.3 8.5c8.6 10.1 7.3 25.3-2.8 33.8l-10 8.5c-14.8 12.5-33.7 19.1-53 18.6c-16.6-.4-30.6 12.4-31.6 29l-1.8 30c-2.5 42.5-38.3 75.3-80.8 74.2c-7.6-.2-15 2.4-20.7 7.3l-10 8.5c-10.1 8.6-25.3 7.3-33.8-2.8s-7.3-25.3 2.8-33.8l10-8.5c14.8-12.5 33.7-19.1 53-18.6c16.6 .4 30.6-12.4 31.6-29l1.8-30c2.5-42.5 38.3-75.3 80.8-74.2c7.6 .2 15-2.4 20.7-7.3l10-8.5c10.1-8.6 25.3-7.3 33.8 2.8zM150.6 201.4l160 160c7.7 7.7 10.9 18.8 8.6 29.4s-9.9 19.4-20 23.2l-39.7 14.9L83.1 252.5 98 212.8c3.8-10.2 12.6-17.7 23.2-20s21.7 1 29.4 8.6zM48.2 345.6l22.6-60.2L226.6 441.2l-60.2 22.6L48.2 345.6zM35.9 378.5l97.6 97.6L43.2 510c-11.7 4.4-25 1.5-33.9-7.3S-2.4 480.5 2 468.8l33.8-90.3z";
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
    exports.faPartyHorn = exports.definition;
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

// src/_entries/faPartyHorn.js
var import_faPartyHorn = __toESM(require_faPartyHorn());
var export_faPartyHorn = import_faPartyHorn.definition;
export {
  export_faPartyHorn as faPartyHorn
};
