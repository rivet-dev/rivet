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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faUserNurseHairLong.js
var require_faUserNurseHairLong = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faUserNurseHairLong.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "user-nurse-hair-long";
    var width = 448;
    var height = 512;
    var aliases = [];
    var unicode = "e45e";
    var svgPathData = "M96 70.2L96 128l.3 0c-.2 2.6-.3 5.3-.3 8l0 24 0 11c0 33.9-13.5 66.5-37.5 90.5l-3.9 3.9c-4.2 4.2-6.6 10-6.6 16C48 293.9 58.1 304 70.6 304L224 304l153.4 0c12.5 0 22.6-10.1 22.6-22.6c0-6-2.4-11.8-6.6-16l-3.9-3.9c-24-24-37.5-56.6-37.5-90.5l0-11 0-24c0-2.7-.1-5.4-.3-8l.3 0 0-57.8c0-13.3-8.3-25.3-20.8-30l-96-36c-7.2-2.7-15.2-2.7-22.5 0l-96 36C104.3 44.9 96 56.8 96 70.2zM304 160l0 16c0 44.2-35.8 80-80 80s-80-35.8-80-80l32 0c29.8 0 55.9-16.3 69.6-40.5C257.3 150.4 275.5 160 296 160l8 0zM151.4 346.8c-6.4-6.8-15.4-10.6-24.6-9.3C55.1 347.7 0 409.4 0 483.8C0 499.4 12.6 512 28.2 512l391.7 0c15.6 0 28.2-12.6 28.2-28.2c0-74.5-55.1-136.1-126.7-146.3c-9.2-1.3-18.3 2.5-24.6 9.3l-61 64.8c-6.3 6.7-17 6.7-23.3 0l-61-64.8zM208 48c0-4.4 3.6-8 8-8l16 0c4.4 0 8 3.6 8 8l0 16 16 0c4.4 0 8 3.6 8 8l0 16c0 4.4-3.6 8-8 8l-16 0 0 16c0 4.4-3.6 8-8 8l-16 0c-4.4 0-8-3.6-8-8l0-16-16 0c-4.4 0-8-3.6-8-8l0-16c0-4.4 3.6-8 8-8l16 0 0-16z";
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
    exports.faUserNurseHairLong = exports.definition;
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

// src/_entries/faUserNurseHairLong.js
var import_faUserNurseHairLong = __toESM(require_faUserNurseHairLong());
var export_faUserNurseHairLong = import_faUserNurseHairLong.definition;
export {
  export_faUserNurseHairLong as faUserNurseHairLong
};
