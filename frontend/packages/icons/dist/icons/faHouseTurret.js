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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faHouseTurret.js
var require_faHouseTurret = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faHouseTurret.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "house-turret";
    var width = 640;
    var height = 512;
    var aliases = [];
    var unicode = "e1b4";
    var svgPathData = "M256 464l0-186.9c0-13.5 5.6-26.3 15.6-35.4l144-132c18.4-16.8 46.5-16.8 64.9 0l144 132c9.9 9.1 15.6 21.9 15.6 35.4L640 464c0 26.5-21.5 48-48 48l-288 0c-26.5 0-48-21.5-48-48zM424 272c-13.3 0-24 10.7-24 24l0 48c0 13.3 10.7 24 24 24l48 0c13.3 0 24-10.7 24-24l0-48c0-13.3-10.7-24-24-24l-48 0zM24 0C10.7 0 0 10.7 0 24L0 96l0 8 0 93.5c0 17 6.7 33.3 18.7 45.3L64 288l0 176c0 26.5 21.5 48 48 48l128 0c-10-13.4-16-30-16-48l0-186.9c0-22.4 9.4-43.8 25.9-59L352 124.6l0-20.6 0-8 0-72c0-13.3-10.7-24-24-24L280 0c-13.3 0-24 10.7-24 24l0 72-32 0 0-72c0-13.3-10.7-24-24-24L152 0c-13.3 0-24 10.7-24 24l0 72L96 96l0-72C96 10.7 85.3 0 72 0L24 0zM176 160c22.1 0 40 17.9 40 40l0 24c0 8.8-7.2 16-16 16l-48 0c-8.8 0-16-7.2-16-16l0-24c0-22.1 17.9-40 40-40z";
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
    exports.faHouseTurret = exports.definition;
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

// src/_entries/faHouseTurret.js
var import_faHouseTurret = __toESM(require_faHouseTurret());
var export_faHouseTurret = import_faHouseTurret.definition;
export {
  export_faHouseTurret as faHouseTurret
};
