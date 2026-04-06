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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faStarfighterTwinIonEngine.js
var require_faStarfighterTwinIonEngine = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faStarfighterTwinIonEngine.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "starfighter-twin-ion-engine";
    var width = 576;
    var height = 512;
    var aliases = ["starfighter-alt"];
    var unicode = "e038";
    var svgPathData = "M32 32c17.7 0 32 14.3 32 32l0 147.2 75.6-15.1C163.3 137.4 220.8 96 288 96s124.7 41.4 148.4 100.1L512 211.2 512 64c0-17.7 14.3-32 32-32s32 14.3 32 32l0 160 0 64 0 160c0 17.7-14.3 32-32 32s-32-14.3-32-32l0-147.2-75.6 15.1C412.7 374.6 355.2 416 288 416s-124.7-41.4-148.4-100.1L64 300.8 64 448c0 17.7-14.3 32-32 32s-32-14.3-32-32L0 288l0-64L0 64C0 46.3 14.3 32 32 32zM272 350.7l0-32.7c-5.9-1.5-11.4-3.8-16.5-6.8l-23.1 23.1c11.6 8.2 25 14 39.6 16.4zm-62.3-39l23.1-23.1c-3-5.1-5.3-10.6-6.8-16.5l-32.7 0c2.4 14.6 8.2 28.1 16.4 39.6zM193.3 240l32.7 0c1.5-5.9 3.8-11.4 6.8-16.5l-23.1-23.1c-8.2 11.6-14 25-16.4 39.6zm39-62.3l23.1 23.1c5.1-3 10.6-5.3 16.5-6.8l0-32.7c-14.6 2.4-28.1 8.2-39.6 16.4zM343.6 334.3l-23.1-23.1c-5.1 3-10.6 5.3-16.5 6.8l0 32.7c14.6-2.4 28.1-8.2 39.6-16.4zm22.6-22.6c8.2-11.6 14-25 16.4-39.6L350 272c-1.5 5.9-3.8 11.4-6.8 16.5l23.1 23.1zM350 240l32.7 0c-2.4-14.6-8.2-28.1-16.4-39.6l-23.1 23.1c3 5.1 5.3 10.6 6.8 16.5zm-6.4-62.3c-11.6-8.2-25-14-39.6-16.4l0 32.7c5.9 1.5 11.4 3.8 16.5 6.8l23.1-23.1zM310.8 278.5c5.7-5.8 9.2-13.7 9.2-22.5c0-17.7-14.3-32-32-32s-32 14.3-32 32s14.3 32 32 32c8.8 0 16.7-3.5 22.5-9.2l.3-.3z";
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
    exports.faStarfighterTwinIonEngine = exports.definition;
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

// src/_entries/faStarfighterAlt.js
var import_faStarfighterTwinIonEngine = __toESM(require_faStarfighterTwinIonEngine());
var export_faStarfighterAlt = import_faStarfighterTwinIonEngine.definition;
export {
  export_faStarfighterAlt as faStarfighterAlt
};
