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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faFarm.js
var require_faFarm = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faFarm.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "farm";
    var width = 576;
    var height = 512;
    var aliases = ["barn-silo"];
    var unicode = "f864";
    var svgPathData = "M0 96C0 43 43 0 96 0s96 43 96 96L0 96zm181.4 32L132.8 246c-3.2 7.7-4.8 16-4.8 24.4L128 480c0 11.7 3.1 22.6 8.6 32L48 512c-26.5 0-48-21.5-48-48L0 128l181.4 0zM160 480l0-209.7c0-4.2 .8-8.3 2.4-12.2l48.9-118.7c3-7.4 8.7-13.3 15.9-16.7L354.3 62.5c8.7-4.1 18.7-4.1 27.4 0l127.1 60.2c7.2 3.4 12.9 9.4 15.9 16.7l48.9 118.7c1.6 3.9 2.4 8 2.4 12.2L576 480c0 17.7-14.3 32-32 32l-112 0 0-64c0-26.5-21.5-48-48-48l-32 0c-26.5 0-48 21.5-48 48l0 64-112 0c-17.7 0-32-14.3-32-32zM320 248l0 48c0 13.3 10.7 24 24 24l48 0c13.3 0 24-10.7 24-24l0-48c0-13.3-10.7-24-24-24l-48 0c-13.3 0-24 10.7-24 24z";
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
    exports.faFarm = exports.definition;
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

// src/_entries/faBarnSilo.js
var import_faFarm = __toESM(require_faFarm());
var export_faBarnSilo = import_faFarm.definition;
export {
  export_faBarnSilo as faBarnSilo
};
