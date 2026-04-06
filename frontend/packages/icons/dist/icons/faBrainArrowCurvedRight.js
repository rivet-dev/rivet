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

// src/node_modules/@fortawesome/pro-solid-svg-icons/faBrainArrowCurvedRight.js
var require_faBrainArrowCurvedRight = __commonJS({
  "src/node_modules/@fortawesome/pro-solid-svg-icons/faBrainArrowCurvedRight.js"(exports) {
    "use strict";
    Object.defineProperty(exports, "__esModule", { value: true });
    var prefix = "fas";
    var iconName = "brain-arrow-curved-right";
    var width = 640;
    var height = 512;
    var aliases = ["mind-share"];
    var unicode = "f677";
    var svgPathData = "M240 56c0-30.9-25.1-56-56-56c-28.9 0-52.7 21.9-55.7 49.9C100.6 56.9 80 82.1 80 112c0 6 .8 11.9 2.4 17.4C53.6 135.7 32 161.3 32 192c0 15 5.1 28.8 13.8 39.7C18.7 244.5 0 272.1 0 304c0 34.2 21.4 63.4 51.6 74.8C49.3 385.4 48 392.6 48 400c0 35.3 28.7 64 64 64c5.6 0 11.1-.7 16.3-2.1c2.9 28.2 26.8 50.1 55.7 50.1c30.9 0 56-25.1 56-56l0-400zm32 0l0 400c0 30.9 25.1 56 56 56c3.2 0 6.4-.3 9.5-.8C321.3 491 304 459.3 304 416c0-79.5 64.5-144 144-144l32 0 0-16c0-4.9 .8-9.8 2.2-14.3c-4.9-3.9-10.2-7.3-16-10C474.9 220.8 480 207 480 192c0-30.7-21.6-56.3-50.4-62.6c1.6-5.5 2.4-11.4 2.4-17.4c0-29.9-20.6-55.1-48.3-62.1C380.6 21.9 356.9 0 328 0c-30.9 0-56 25.1-56 56zM539.3 244.7c-4.6-4.6-11.5-5.9-17.4-3.5s-9.9 8.3-9.9 14.8l0 48-64 0c-61.9 0-112 50.1-112 112c0 54.3 34.6 85.6 45 93.8c1.9 1.5 4.3 2.2 6.7 2.2c6.8 0 12.3-5.5 12.3-12.3l0-35.7c0-35.3 28.7-64 64-64l48 0 0 48c0 6.5 3.9 12.3 9.9 14.8s12.9 1.1 17.4-3.5l96-96c6.2-6.2 6.2-16.4 0-22.6l-96-96z";
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
    exports.faBrainArrowCurvedRight = exports.definition;
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

// src/_entries/faBrainArrowCurvedRight.js
var import_faBrainArrowCurvedRight = __toESM(require_faBrainArrowCurvedRight());
var export_faBrainArrowCurvedRight = import_faBrainArrowCurvedRight.definition;
export {
  export_faBrainArrowCurvedRight as faBrainArrowCurvedRight
};
